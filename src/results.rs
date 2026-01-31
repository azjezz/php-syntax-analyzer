use std::collections::{HashMap, HashSet};

use cli_table::{Cell, Style, Table, format::Justify, print_stdout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vendor {
    Symfony,
    Laravel,
    Doctrine,
    Phpunit,
    Twig,
    Illuminate,
    Other,
}

impl Vendor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Vendor::Symfony => "symfony/",
            Vendor::Laravel => "laravel/",
            Vendor::Doctrine => "doctrine/",
            Vendor::Phpunit => "phpunit/",
            Vendor::Twig => "twig/",
            Vendor::Illuminate => "illuminate/",
            Vendor::Other => "",
        }
    }

    pub const fn is_well_known(&self) -> bool {
        !matches!(self, Vendor::Other)
    }

    pub fn from_package(package: &str) -> Self {
        if package.starts_with("symfony/") {
            Vendor::Symfony
        } else if package.starts_with("laravel/") {
            Vendor::Laravel
        } else if package.starts_with("doctrine/") {
            Vendor::Doctrine
        } else if package.starts_with("phpunit/") {
            Vendor::Phpunit
        } else if package.starts_with("twig/") {
            Vendor::Twig
        } else if package.starts_with("illuminate/") {
            Vendor::Illuminate
        } else {
            Vendor::Other
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeywordMatch {
    pub keyword: String,
    pub vendor: Vendor,
    pub is_hard: bool,
}

#[derive(Debug, Clone)]
pub struct LabelMatch {
    // todo: this sohuld be &'arena str, no need to re-allocate
    pub label: String,
    pub vendor: Vendor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl ImpactLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImpactLevel::None => "None",
            ImpactLevel::Low => "Low",
            ImpactLevel::Medium => "Medium",
            ImpactLevel::High => "High",
            ImpactLevel::Critical => "Critical",
        }
    }

    pub fn calculate(total: usize) -> Self {
        match total {
            0 => ImpactLevel::None,
            1..=25 => ImpactLevel::Low,
            26..=100 => ImpactLevel::Medium,
            101..=500 => ImpactLevel::High,
            _ => ImpactLevel::Critical,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeywordResult {
    pub soft_count: usize,
    pub hard_count: usize,
    pub well_known_vendors: HashSet<Vendor>,
}

#[derive(Debug, Clone)]
pub struct LabelResult {
    pub count: usize,
    pub well_known_vendors: HashSet<Vendor>,
}

impl KeywordResult {
    pub fn new() -> Self {
        Self {
            soft_count: 0,
            hard_count: 0,
            well_known_vendors: HashSet::new(),
        }
    }

    pub fn total_count(&self) -> usize {
        self.soft_count + self.hard_count
    }

    pub fn soft_impact(&self) -> ImpactLevel {
        ImpactLevel::calculate(self.soft_count)
    }

    pub fn hard_impact(&self) -> ImpactLevel {
        ImpactLevel::calculate(self.total_count())
    }

    pub fn add_match(&mut self, m: &KeywordMatch) {
        if m.is_hard {
            self.hard_count += 1;
        } else {
            self.soft_count += 1;
        }

        if m.vendor.is_well_known() {
            self.well_known_vendors.insert(m.vendor);
        }
    }
}

impl LabelResult {
    pub fn new() -> Self {
        Self {
            count: 0,
            well_known_vendors: HashSet::new(),
        }
    }

    pub fn add_match(&mut self, m: &LabelMatch) {
        self.count += 1;
        if m.vendor.is_well_known() {
            self.well_known_vendors.insert(m.vendor);
        }
    }
}

#[derive(Debug)]
pub struct AnalysisReport {
    pub keyword_results: HashMap<String, KeywordResult>,
    pub label_results: HashMap<String, LabelResult>,
    pub total_files: usize,
}

impl AnalysisReport {
    pub fn new(total_files: usize) -> Self {
        Self {
            keyword_results: HashMap::new(),
            label_results: HashMap::new(),
            total_files,
        }
    }

    pub fn add_keyword_matches(&mut self, matches: Vec<KeywordMatch>) {
        for m in matches {
            self.keyword_results
                .entry(m.keyword.clone())
                .or_insert_with(KeywordResult::new)
                .add_match(&m);
        }
    }

    pub fn add_label_matches(&mut self, matches: Vec<LabelMatch>) {
        for m in matches {
            self.label_results
                .entry(m.label.clone())
                .or_insert_with(LabelResult::new)
                .add_match(&m);
        }
    }

    pub fn ensure_all_keywords(&mut self, keywords: &[String]) {
        for keyword in keywords {
            self.keyword_results
                .entry(keyword.clone())
                .or_insert_with(KeywordResult::new);
        }
    }

    pub fn should_warn_low_file_count(&self) -> bool {
        self.total_files < 200_000
    }

    fn wrap_text(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            return text.to_string();
        }

        let mut result = String::new();
        let mut current_line = String::new();

        for word in text.split(", ") {
            let word_with_sep = if current_line.is_empty() {
                word.to_string()
            } else {
                format!(", {}", word)
            };

            if current_line.len() + word_with_sep.len() <= max_width {
                current_line.push_str(&word_with_sep);
            } else {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&current_line);
        }

        result
    }

    fn create_impact_cell(impact: ImpactLevel) -> cli_table::CellStruct {
        match impact {
            ImpactLevel::None => impact
                .as_str()
                .cell()
                .foreground_color(Some(cli_table::Color::Green)),
            ImpactLevel::Low => impact
                .as_str()
                .cell()
                .foreground_color(Some(cli_table::Color::Cyan)),
            ImpactLevel::Medium => impact
                .as_str()
                .cell()
                .foreground_color(Some(cli_table::Color::Yellow)),
            ImpactLevel::High => impact
                .as_str()
                .cell()
                .foreground_color(Some(cli_table::Color::Red)),
            ImpactLevel::Critical => impact
                .as_str()
                .cell()
                .foreground_color(Some(cli_table::Color::Magenta))
                .bold(true),
        }
    }

    pub fn display_table(&self, show_keywords: bool, show_labels: bool) {
        if self.should_warn_low_file_count() {
            eprintln!(
                "\n⚠️  WARNING: Only analyzed {} files (less than 200,000 recommended)",
                self.total_files
            );
            eprintln!(
                "   Consider increasing --max to scan more packages for a comprehensive analysis.\n"
            );
        }

        if show_keywords {
            let mut keyword_data: Vec<_> = self
                .keyword_results
                .iter()
                .map(|(keyword, result)| {
                    let soft_impact = result.soft_impact();
                    let hard_impact = result.hard_impact();
                    (keyword.clone(), result, soft_impact, hard_impact)
                })
                .collect();

            if keyword_data.is_empty() {
                tracing::info!("No keywords match found in the analyzed packages.");
                return;
            }

            keyword_data.sort_by(|a, b| {
                b.3.cmp(&a.3)
                    .then_with(|| b.1.total_count().cmp(&a.1.total_count()))
                    .then_with(|| a.0.cmp(&b.0))
            });

            let mut keyboard_rows = Vec::new();
            for (keyword, result, soft_impact, hard_impact) in keyword_data {
                let well_known_str = if result.well_known_vendors.is_empty() {
                    "-".to_string()
                } else {
                    let mut vendors: Vec<_> = result
                        .well_known_vendors
                        .iter()
                        .map(|v| v.as_str().trim_end_matches('/'))
                        .collect();
                    vendors.sort();
                    Self::wrap_text(&vendors.join(", "), 60)
                };

                keyboard_rows.push(vec![
                    keyword.cell().bold(true),
                    result.soft_count.cell().justify(Justify::Right),
                    result.hard_count.cell().justify(Justify::Right),
                    Self::create_impact_cell(soft_impact),
                    Self::create_impact_cell(hard_impact),
                    well_known_str.cell(),
                ]);
            }

            let table = keyboard_rows.table().title(vec![
                "Keyword".cell().bold(true),
                "Soft".cell().bold(true),
                "Hard".cell().bold(true),
                "Soft Impact".cell().bold(true),
                "Hard Impact".cell().bold(true),
                "Well-Known Vendors".cell().bold(true),
            ]);

            let _ = print_stdout(table);
        }

        if show_labels {
            if show_keywords {
                println!();
            }

            let mut label_data: Vec<_> = self.label_results.iter().collect();
            if label_data.is_empty() {
                tracing::info!("No labels match found in the analyzed packages.");
                return;
            }

            label_data.sort_by(|a, b| a.0.cmp(&b.0));

            let mut label_rows = Vec::new();
            for (label, result) in label_data {
                let well_known_str = if result.well_known_vendors.is_empty() {
                    "-".to_string()
                } else {
                    let mut vendors: Vec<_> = result
                        .well_known_vendors
                        .iter()
                        .map(|v| v.as_str().trim_end_matches('/'))
                        .collect();
                    vendors.sort();
                    Self::wrap_text(&vendors.join(", "), 60)
                };

                label_rows.push(vec![
                    label.cell().bold(true),
                    result.count.cell().justify(Justify::Right),
                    well_known_str.cell(),
                ]);
            }

            let label_table = label_rows.table().title(vec![
                "Label".cell().bold(true),
                "Count".cell().bold(true),
                "Well-Known Vendors".cell().bold(true),
            ]);

            let _ = print_stdout(label_table);
        }
    }
}
