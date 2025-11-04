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
    target: AnalysisTargetKeyword,
) -> Result<(), DatabaseError> {
    tracing::info!(
        "Starting analysis for keyword '{}' in directory {:?}",
        target.as_str(),
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

    tracing::info!("Analyzing {} files...", files.len());

    let issues: IssueCollection = files
        .into_par_iter()
        .map_init(Bump::new, |arena, file| Analyzer::run(arena, &file, target))
        .reduce(IssueCollection::new, |mut acc, coll| {
            acc.extend(coll);
            acc
        });

    let reporter = Reporter::new(
        database.read_only(),
        ReportingTarget::Stdout,
        ColorChoice::Always,
        false,
        None,
    );

    reporter
        .report(issues, ReportingFormat::Rich)
        .expect("Failed to report issues");

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisTargetKeyword {
    Let,
    Scope,
    Using,
}

impl AnalysisTargetKeyword {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnalysisTargetKeyword::Let => "let",
            AnalysisTargetKeyword::Scope => "scope",
            AnalysisTargetKeyword::Using => "using",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Analyzer {
    target_keyword: AnalysisTargetKeyword,
}

impl Analyzer {
    pub fn run<'arena, 'ctx>(
        arena: &'arena Bump,
        file: &'ctx File,
        target_keyword: AnalysisTargetKeyword,
    ) -> IssueCollection {
        let (program, _) = parse_file(arena, file);

        let resolved_names = NameResolver::new(arena).resolve(program);
        let mut ctx = AnalysisContext::new(arena, file, program, resolved_names);
        let analyzer = Analyzer { target_keyword };
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

impl<'ctx, 'ast, 'arena> Walker<'ast, 'arena, AnalysisContext<'ctx, 'arena>> for Analyzer {
    fn walk_in_function_call(
        &self,
        function_call: &'ast FunctionCall<'arena>,
        ctx: &mut AnalysisContext<'ctx, 'arena>,
    ) {
        let Expression::Identifier(identifier) = function_call.function else {
            return;
        };

        let resolved_name = ctx.resolved_names.get(identifier);

        let last_segment = resolved_name.split('\\').last().unwrap_or_default();
        if !last_segment.eq_ignore_ascii_case(self.target_keyword.as_str()) {
            return;
        }

        ctx.collector.report(
            Issue::error("Found usage of target keyword").with_annotation(
                Annotation::primary(identifier.span())
                    .with_message(format!("Call to function `{resolved_name}` found here")),
            ),
        );
    }
}
