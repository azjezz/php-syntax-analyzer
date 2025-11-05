use std::path::{Path, PathBuf};

use anyhow::Result;
use bumpalo::Bump;
use rayon::prelude::*;

use mago_names::ResolvedNames;
use mago_names::resolver::NameResolver;
use mago_span::HasPosition;
use mago_syntax::ast::*;
use mago_syntax::parser::parse_file;
use mago_syntax::walker::Walker;

use crate::files::{read_file, walk_files};
use crate::results::{AnalysisReport, Match, Vendor};

#[tracing::instrument(name = "analyzing-directory")]
pub fn analyze_directory(
    sources_directory: PathBuf,
    keywords: Vec<String>,
) -> Result<AnalysisReport> {
    tracing::info!("Starting analysis...");

    let sources_canonical = sources_directory.canonicalize()?;

    let keyword_refs: Vec<&str> = keywords.iter().map(|s| s.as_str()).collect();
    let all_matches: Vec<Vec<Match>> = walk_files(&sources_canonical)
        .map_init(Bump::new, |arena, file| {
            Analyzer::run(arena, &file, &sources_canonical, &keyword_refs)
        })
        .collect();

    tracing::info!("Collected matches from {} files.", all_matches.len());

    let mut report = AnalysisReport::new(all_matches.len());
    let matches: Vec<Match> = all_matches.into_iter().flatten().collect();
    report.add_matches(matches);
    report.ensure_all_keywords(&keywords);

    tracing::info!("Analysis complete.");

    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analyzer<'ctx> {
    hard: bool,
    keywords: &'ctx [&'ctx str],
}

impl<'ctx> Analyzer<'ctx> {
    #[tracing::instrument(name = "analyzing-file", skip(arena, sources_canonical, keywords))]
    pub fn run<'arena>(
        arena: &'arena Bump,
        file: &Path,
        sources_canonical: &Path,
        keywords: &'ctx [&'ctx str],
    ) -> Vec<Match> {
        let Some((vendor, file)) = read_file(file, sources_canonical) else {
            return Vec::with_capacity(0);
        };

        let (program, _) = parse_file(arena, &file);
        let resolved_names = NameResolver::new(arena).resolve(program);
        let mut ctx = AnalysisContext::new(vendor, resolved_names);
        let analyzer = Analyzer {
            hard: true,
            keywords,
        };
        analyzer.walk_program(program, &mut ctx);
        ctx.matches
    }
}

pub struct AnalysisContext<'arena> {
    vendor: Vendor,
    resolved_names: ResolvedNames<'arena>,
    matches: Vec<Match>,
}

impl<'arena> AnalysisContext<'arena> {
    pub fn new(vendor: Vendor, resolved_names: ResolvedNames<'arena>) -> Self {
        Self {
            vendor,
            resolved_names,
            matches: Vec::new(),
        }
    }
}

impl<'ctx, 'ast, 'arena> Walker<'ast, 'arena, AnalysisContext<'arena>> for Analyzer<'ctx> {
    fn walk_in_function_call(
        &self,
        function_call: &'ast FunctionCall<'arena>,
        ctx: &mut AnalysisContext<'arena>,
    ) {
        let Expression::Identifier(identifier) = function_call.function else {
            return;
        };

        let resolved_name = ctx.resolved_names.get(identifier);
        let last_segment = resolved_name.split('\\').next_back().unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                ctx.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: ctx.vendor,
                    is_hard: false,
                });

                break;
            }
        }
    }

    fn walk_in_function_closure_creation(
        &self,
        function_closure_creation: &'ast FunctionClosureCreation<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        let Expression::Identifier(identifier) = function_closure_creation.function else {
            return;
        };

        let resolved_name = context.resolved_names.get(identifier);
        let last_segment = resolved_name.split('\\').next_back().unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                context.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: false,
                });

                break;
            }
        }
    }

    fn walk_in_function(
        &self,
        function: &'ast Function<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        let name = &function.name.value;

        for &keyword in self.keywords {
            if name.eq_ignore_ascii_case(keyword) {
                context.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: false,
                });

                break;
            }
        }
    }

    fn walk_in_local_identifier(
        &self,
        local_identifier: &'ast LocalIdentifier<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        if !self.hard {
            return;
        }

        for &keyword in self.keywords {
            if local_identifier.value.eq_ignore_ascii_case(keyword) {
                context.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: true,
                });

                break;
            }
        }
    }

    fn walk_in_qualified_identifier(
        &self,
        qualified_identifier: &'ast QualifiedIdentifier<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        if !self.hard {
            return;
        }

        let position = qualified_identifier.position();
        if !context.resolved_names.contains(&position) {
            return;
        }

        let last_segment = context
            .resolved_names
            .get(&position)
            .split('\\')
            .next_back()
            .unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                context.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: true,
                });

                break;
            }
        }
    }

    fn walk_in_fully_qualified_identifier(
        &self,
        fully_qualified_identifier: &'ast FullyQualifiedIdentifier<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        if !self.hard {
            return;
        }

        let last_segment = fully_qualified_identifier
            .value
            .split('\\')
            .next_back()
            .unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                context.matches.push(Match {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: true,
                });

                break;
            }
        }
    }
}
