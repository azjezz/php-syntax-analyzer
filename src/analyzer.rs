use std::path::PathBuf;

use bumpalo::Bump;
use rayon::prelude::*;

use mago_collector::Collector;
use mago_database::DatabaseReader;
use mago_database::error::DatabaseError;
use mago_database::file::File;
use mago_database::loader::DatabaseLoader;
use mago_names::ResolvedNames;
use mago_names::resolver::NameResolver;
use mago_reporting::Annotation;
use mago_reporting::ColorChoice;
use mago_reporting::Issue;
use mago_reporting::IssueCollection;
use mago_reporting::reporter::Reporter;
use mago_reporting::reporter::ReportingFormat;
use mago_reporting::reporter::ReportingTarget;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::parser::parse_file;
use mago_syntax::walker::Walker;

pub fn analyze_directory(
    donwload_directory: PathBuf,
    sources_directory: PathBuf,
    keywords: Vec<String>,
    display: bool,
) -> Result<(), DatabaseError> {
    tracing::info!(
        "Starting analysis for keywords '{:?}' in directory {:?}",
        keywords,
        donwload_directory
    );

    let loader = DatabaseLoader::new(
        donwload_directory.canonicalize()?,
        vec![sources_directory.canonicalize()?],
        vec![],
        vec![],
        vec!["php".to_string(), "php7".to_string(), "php8".to_string()],
    );
    let database = loader.load()?;
    let files = database.files().collect::<Vec<_>>();

    tracing::info!(
        "Analyzing {} files for {} keywords...",
        files.len(),
        keywords.len()
    );

    let keyword_refs: Vec<&str> = keywords.iter().map(|s| s.as_str()).collect();
    let all_issues: Vec<IssueCollection> = files
        .par_iter()
        .map_init(Bump::new, |arena, file| {
            Analyzer::run(arena, file, &keyword_refs)
        })
        .collect();

    // Separate issues by keyword
    for keyword in &keywords {
        let keyword_issues: IssueCollection = all_issues
            .iter()
            .flat_map(|issues| issues.iter())
            .filter(|issue| issue.message.contains(&format!("keyword '{}'", keyword)))
            .cloned()
            .collect();

        let issue_count = keyword_issues.len();
        if issue_count == 0 {
            tracing::info!("No issues found for keyword '{keyword}'.");
            continue;
        }

        tracing::error!("Analysis complete for keyword '{keyword}'. Found {issue_count} issues.");
        if !display {
            continue;
        }

        let reporter = Reporter::new(
            database.read_only(),
            ReportingTarget::Stdout,
            ColorChoice::Always,
            false,
            None,
        );

        reporter
            .report(keyword_issues, ReportingFormat::Rich)
            .expect("Failed to report issues");
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analyzer<'ctx> {
    keywords: &'ctx [&'ctx str],
}

impl<'ctx> Analyzer<'ctx> {
    pub fn run<'arena>(
        arena: &'arena Bump,
        file: &File,
        keywords: &'ctx [&'ctx str],
    ) -> IssueCollection {
        let (program, _) = parse_file(arena, file);

        let resolved_names = NameResolver::new(arena).resolve(program);
        let mut ctx = AnalysisContext::new(arena, file, program, resolved_names);
        let analyzer = Analyzer { keywords };
        analyzer.walk_program(program, &mut ctx);
        ctx.collector.finish()
    }
}

pub struct AnalysisContext<'ctx, 'arena> {
    resolved_names: ResolvedNames<'arena>,
    collector: Collector<'ctx, 'arena>,
}

impl<'ctx, 'arena> AnalysisContext<'ctx, 'arena> {
    pub fn new<'ast>(
        arena: &'arena Bump,
        file: &'ctx File,
        program: &'ast Program<'arena>,
        resolved_names: ResolvedNames<'arena>,
    ) -> Self {
        Self {
            resolved_names,
            collector: Collector::new(arena, file, program, &[]),
        }
    }
}

impl<'ctx, 'ast, 'arena> Walker<'ast, 'arena, AnalysisContext<'ctx, 'arena>> for Analyzer<'ctx> {
    fn walk_in_function_call(
        &self,
        function_call: &'ast FunctionCall<'arena>,
        ctx: &mut AnalysisContext<'ctx, 'arena>,
    ) {
        let Expression::Identifier(identifier) = function_call.function else {
            return;
        };

        let resolved_name = ctx.resolved_names.get(identifier);

        let last_segment = resolved_name.split('\\').next_back().unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                ctx.collector.report(
                    Issue::error(format!("Found usage of target keyword '{}'", keyword))
                        .with_annotation(Annotation::primary(identifier.span()).with_message(
                            format!("Call to function `{resolved_name}` found here"),
                        )),
                );

                break;
            }
        }
    }

    fn walk_in_function_closure_creation(
        &self,
        function_closure_creation: &'ast FunctionClosureCreation<'arena>,
        context: &mut AnalysisContext<'ctx, 'arena>,
    ) {
        let Expression::Identifier(identifier) = function_closure_creation.function else {
            return;
        };

        let resolved_name = context.resolved_names.get(identifier);
        let last_segment = resolved_name.split('\\').next_back().unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                context.collector.report(
                    Issue::error(format!("Found usage of target keyword '{}'", keyword))
                        .with_annotation(Annotation::primary(identifier.span()).with_message(
                            format!("Function closure creation for `{resolved_name}` found here"),
                        )),
                );

                break;
            }
        }
    }

    fn walk_in_function(
        &self,
        function: &'ast Function<'arena>,
        context: &mut AnalysisContext<'ctx, 'arena>,
    ) {
        let name = &function.name.value;

        for &keyword in self.keywords {
            if name.eq_ignore_ascii_case(keyword) {
                let fqn = context.resolved_names.get(&function.name);

                context.collector.report(
                    Issue::error(format!("Found usage of target keyword '{}'", keyword))
                        .with_annotation(
                            Annotation::primary(function.name.span())
                                .with_message(format!("Function `{name}` found here")),
                        )
                        .with_annotation(
                            Annotation::secondary(function.span())
                                .with_message(format!("Function `{fqn}` defined here")),
                        ),
                );

                break;
            }
        }
    }
}
