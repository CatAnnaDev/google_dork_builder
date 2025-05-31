mod templates;

use copypasta::{ClipboardContext, ClipboardProvider};
use eframe::egui;
use open;
use serde::{Deserialize, Serialize};
use std::fs;
use crate::templates::TEMPLATES;

const HISTORY_FILE: &str = "dork_history.json";

#[derive(Serialize, Deserialize, Clone)]
struct DorkTemplate {
    name: &'static str,
    category: &'static str,
    site: &'static str,
    inurl: &'static str,
    intitle: &'static str,
    filetype: &'static str,
    intext: &'static str,
}

impl DorkTemplate {
    fn matches_category(&self, category: &str) -> bool {
        self.category.eq_ignore_ascii_case(category)
    }
}

fn filter_dorks_by_category(dorks: &[DorkTemplate], category: &str) -> Vec<DorkTemplate> {
    dorks
        .iter()
        .filter(|d| d.matches_category(category))
        .cloned()
        .collect()
}

fn unique_categories(dorks: &[DorkTemplate]) -> Vec<String> {
    let mut categories: Vec<String> = dorks.iter().map(|d| d.category.to_string()).collect();
    categories.sort_unstable();
    categories.dedup();
    categories
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct DorkData {
    site: String,
    inurl: String,
    intitle: String,
    filetype: String,
    intext: String,
    operator: String,
}

struct DorkApp {
    data: DorkData,
    query: String,
    history: Vec<String>,
    selected_template: usize,
    selected_history: usize,
    available_operators: Vec<&'static str>,
    pub selected_category: String,
}

impl DorkApp {
    fn generate_query(&mut self) {
        let mut parts = vec![];

        let op = self.data.operator.trim();
        let glue = if op.is_empty() {
            " "
        } else {
            &*format!(" {} ", op)
        };

        if !self.data.site.is_empty() {
            parts.push(format!("site:{}", self.data.site));
        }
        if !self.data.inurl.is_empty() {
            parts.push(format!("inurl:\"{}\"", self.data.inurl));
        }
        if !self.data.intitle.is_empty() {
            parts.push(format!("intitle:\"{}\"", self.data.intitle));
        }
        if !self.data.filetype.is_empty() {
            parts.push(format!("filetype:{}", self.data.filetype));
        }
        if !self.data.intext.is_empty() {
            parts.push(format!("intext:\"{}\"", self.data.intext));
        }

        self.query = parts.join(&glue);

        if !self.query.is_empty() && !self.history.contains(&self.query) {
            self.history.push(self.query.clone());
            let _ = self.save_history();
        }
    }

    fn save_history(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self.history)?;
        fs::write(HISTORY_FILE, json)
    }

    fn load_history(&mut self) {
        if let Ok(content) = fs::read_to_string(HISTORY_FILE) {
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&content) {
                self.history = parsed;
            }
        }
    }

    fn apply_template(&mut self, index: usize) {
        let tpl = &TEMPLATES[index];
        self.data.inurl = tpl.inurl.to_string();
        self.data.intitle = tpl.intitle.to_string();
        self.data.filetype = tpl.filetype.to_string();
        self.data.site = tpl.site.to_string();
        self.data.intext = tpl.intext.to_string();
    }

    fn apply_query_string(&mut self, query: &str) {
        self.data.inurl.clear();
        self.data.intitle.clear();
        self.data.filetype.clear();
        self.data.site.clear();

        for token in query.split_whitespace() {
            if let Some(rest) = token.strip_prefix("inurl:") {
                self.data.inurl = rest.to_string();
            } else if let Some(rest) = token.strip_prefix("intitle:") {
                self.data.intitle = rest.to_string();
            } else if let Some(rest) = token.strip_prefix("filetype:") {
                self.data.filetype = rest.to_string();
            } else if let Some(rest) = token.strip_prefix("site:") {
                self.data.site = rest.to_string();
            } else {
                if !self.data.intext.is_empty() {
                    self.data.intext.push(' ');
                }
                self.data.intext.push_str(token);
            }
        }
    }

    fn default() -> Self {
        Self {
            data: Default::default(),
            query: "".to_string(),
            history: vec![],
            selected_template: 0,
            selected_history: 0,
            available_operators: vec![],
            selected_category: "".to_string(),
        }
    }
}

impl eframe::App for DorkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🔍 Google Dork Builder");
            
            let previous = self.selected_template;
            let prev_history = self.selected_history;
            let prev_category = self.selected_category.clone();
            
            ui.horizontal(|ui| {
                ui.label("Catégorie:");
                egui::ComboBox::from_id_salt("category_select")
                    .selected_text(&self.selected_category)
                    .show_ui(ui, |ui| {
                        for category in unique_categories(TEMPLATES) {
                            ui.selectable_value(&mut self.selected_category, category.clone(), category);
                            if self.selected_category != prev_category {
                                self.selected_template = 0;
                            }
                        }
                    });
            });

            let filtered_templates = filter_dorks_by_category(TEMPLATES, &self.selected_category);

            ui.horizontal(|ui| {
                ui.label("Template:");
                egui::ComboBox::from_id_salt("template_select")
                    .selected_text(
                        filtered_templates
                            .get(self.selected_template)
                            .map(|tpl| tpl.name)
                            .unwrap_or("Aucun"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, tpl) in filtered_templates.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_template, i, tpl.name);
                        }
                    });
            });

            if self.selected_template != previous {
                self.apply_template(self.selected_template);
                self.generate_query();
            }

            ui.separator();

            ui.label("Opérateur logique (ex: AND / OR / -) :");
            ui.text_edit_singleline(&mut self.data.operator);

            ui.label("site:");
            ui.text_edit_singleline(&mut self.data.site);

            ui.label("inurl:");
            ui.text_edit_singleline(&mut self.data.inurl);

            ui.label("intitle:");
            ui.text_edit_singleline(&mut self.data.intitle);

            ui.label("filetype:");
            ui.text_edit_singleline(&mut self.data.filetype);

            ui.label("intext:");
            ui.text_edit_singleline(&mut self.data.intext);

            if ui.button("🔧 Générer la requête").clicked() {
                self.generate_query();
            }

            ui.separator();

            ui.label("🕓 Historique des requêtes :");
            egui::ComboBox::from_id_salt("history_select")
                .selected_text(self.history.get(self.selected_history).unwrap_or(&"".to_string()))
                .show_ui(ui, |ui| {
                    for (i, entry) in self.history.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_history, i, entry);
                    }
                });
            
            if self.selected_history != prev_history {
                if let Some(query) = self.history.get(self.selected_history) {
                    let query = query.clone(); // clone la String
                    self.apply_query_string(&query);
                    self.generate_query();
                }
            }

            ui.label("🔎 Requête générée :");
            ui.text_edit_multiline(&mut self.query);

            ui.horizontal(|ui| {
                if ui.button("📋 Copier").clicked() {
                    let mut ctx = ClipboardContext::new().unwrap();
                    let _ = ctx.set_contents(self.query.clone());
                }

                if ui.button("🌐 Ouvrir dans le navigateur").clicked() {
                    let encoded = urlencoding::encode(&self.query);
                    let _ = open::that(format!("https://www.google.com/search?q={}", encoded));
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    let mut app = DorkApp::default();
    app.load_history();
    eframe::run_native(
        "Google Dork Builder",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
