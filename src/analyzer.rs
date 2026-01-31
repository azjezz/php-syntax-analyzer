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
use crate::results::{AnalysisReport, KeywordMatch, LabelMatch, Vendor};

#[tracing::instrument(name = "analyzing-directory")]
pub fn analyze_directory(
    sources_directory: PathBuf,
    keywords: Vec<String>,
    labels: Vec<String>,
) -> Result<AnalysisReport> {
    tracing::info!("Starting analysis...");

    let sources_canonical = sources_directory.canonicalize()?;

    let keyword_refs: Vec<&str> = keywords.iter().map(|s| s.as_str()).collect();
    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let all_matches: Vec<(Vec<KeywordMatch>, Vec<LabelMatch>)> = walk_files(&sources_canonical)
        .map_init(Bump::new, |arena, file| {
            Analyzer::run(arena, &file, &sources_canonical, &keyword_refs, &label_refs)
        })
        .collect();

    tracing::info!("Collected matches from {} files.", all_matches.len());

    let mut report = AnalysisReport::new(all_matches.len());
    let mut keyword_matches = Vec::new();
    let mut label_matches = Vec::new();
    for (keywords, labels) in all_matches {
        keyword_matches.extend(keywords);
        label_matches.extend(labels);
    }

    report.add_keyword_matches(keyword_matches);
    report.add_label_matches(label_matches);

    report.ensure_all_keywords(&keywords);

    tracing::info!("Analysis complete.");

    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analyzer<'ctx> {
    hard: bool,
    keywords: &'ctx [&'ctx str],
    labels: &'ctx [&'ctx str],
}

impl<'ctx> Analyzer<'ctx> {
    #[tracing::instrument(name = "analyzing-file", skip(arena, sources_canonical, keywords))]
    pub fn run<'arena>(
        arena: &'arena Bump,
        file: &Path,
        sources_canonical: &Path,
        keywords: &'ctx [&'ctx str],
        labels: &'ctx [&'ctx str],
    ) -> (Vec<KeywordMatch>, Vec<LabelMatch>) {
        let Some((vendor, file)) = read_file(file, sources_canonical) else {
            return (Vec::new(), Vec::new());
        };

        let (program, _) = parse_file(arena, &file);
        let resolved_names = NameResolver::new(arena).resolve(program);
        let mut ctx = AnalysisContext::new(vendor, resolved_names);
        let analyzer = Analyzer {
            hard: true,
            keywords,
            labels,
        };
        analyzer.walk_program(program, &mut ctx);

        (ctx.keyword_matches, ctx.label_matches)
    }
}

pub struct AnalysisContext<'arena> {
    vendor: Vendor,
    resolved_names: ResolvedNames<'arena>,
    keyword_matches: Vec<KeywordMatch>,
    label_matches: Vec<LabelMatch>,
}

impl<'arena> AnalysisContext<'arena> {
    pub fn new(vendor: Vendor, resolved_names: ResolvedNames<'arena>) -> Self {
        Self {
            vendor,
            resolved_names,
            keyword_matches: Vec::new(),
            label_matches: Vec::new(),
        }
    }
}

impl<'ctx, 'ast, 'arena> Walker<'ast, 'arena, AnalysisContext<'arena>> for Analyzer<'ctx> {
    fn walk_in_label(&self, label: &'ast Label<'arena>, ctx: &mut AnalysisContext<'arena>) {
        for label_v in self.labels {
            if label.name.value.eq_ignore_ascii_case(label_v) {
                ctx.label_matches.push(LabelMatch {
                    label: label.name.value.to_string(),
                    vendor: ctx.vendor,
                });
            }
        }
    }

    fn walk_in_named_argument(
        &self,
        named_argument: &'ast NamedArgument<'arena>,
        ctx: &mut AnalysisContext<'arena>,
    ) {
        for label_v in self.labels {
            if named_argument.name.value.eq_ignore_ascii_case(label_v) {
                ctx.label_matches.push(LabelMatch {
                    label: named_argument.name.value.to_string(),
                    vendor: ctx.vendor,
                });
            }
        }
    }

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
                ctx.keyword_matches.push(KeywordMatch {
                    keyword: keyword.to_string(),
                    vendor: ctx.vendor,
                    is_hard: false,
                });

                break;
            }
        }
    }

    fn walk_in_function_partial_application(
        &self,
        function_partial_application: &'ast FunctionPartialApplication<'arena>,
        context: &mut AnalysisContext<'arena>,
    ) {
        let Expression::Identifier(identifier) = function_partial_application.function else {
            return;
        };

        let resolved_name = context.resolved_names.get(identifier);
        let last_segment = resolved_name.split('\\').next_back().unwrap_or_default();

        for &keyword in self.keywords {
            if last_segment.eq_ignore_ascii_case(keyword) {
                context.keyword_matches.push(KeywordMatch {
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
                context.keyword_matches.push(KeywordMatch {
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
                context.keyword_matches.push(KeywordMatch {
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
                context.keyword_matches.push(KeywordMatch {
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
                context.keyword_matches.push(KeywordMatch {
                    keyword: keyword.to_string(),
                    vendor: context.vendor,
                    is_hard: true,
                });

                break;
            }
        }
    }
}
