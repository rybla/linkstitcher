//! This module contains utilities for fetching article information from
//! [ArXiv](https://arxiv.org).
//!
//! This module was adapted from from arxiv-rs by Jun Hirako.
//!   - repository: https://github.com/moisutsu/arxiv-rs
//!   - author: https://github.com/moisutsu
use anyhow::{Result, anyhow};
use map_macro::hash_map;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use xml::EventReader;
use xml::reader::XmlEvent;

pub async fn fetch_by_url(arxiv_url: &str) -> Result<Arxiv> {
    let arxiv_id = get_id_from_url(arxiv_url).ok_or_else(|| anyhow!("Invalid ArXiv URL"))?;
    fetch_by_id(arxiv_id).await
}

pub fn get_id_from_url(url: &str) -> Option<&str> {
    if let Some(s) = url.strip_prefix("https://arxiv.org/pdf/") {
        Some(s.strip_suffix(".pdf").unwrap_or(s))
    } else if let Some(s) = url.strip_prefix("https://arxiv.org/abs/") {
        Some(s)
    } else if let Some(s) = url.strip_prefix("https://arxiv.org/html/") {
        Some(s)
    } else {
        None
    }
}

pub async fn fetch_by_id(arxiv_id: &str) -> Result<Arxiv> {
    let query = ArxivQueryBuilder::new().id_list(arxiv_id).build();
    let result = fetch_arxivs(query).await?;
    if let Some(result) = result.first() {
        Ok(result.clone())
    } else {
        Err(anyhow!("no ArXiv results"))
    }
}

/// A structure that stores the paper information.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Arxiv {
    pub id: String,
    pub updated: String,
    pub published: String,
    pub title: String,
    pub summary: String,
    pub authors: Vec<String>,
    pub primary_category: String,
    pub primary_category_name: String,
    pub category_names: Vec<String>,
    pub categories: Vec<String>,
    pub pdf_url: String,
    pub html_url: String,
    pub comment: Option<String>,
}

impl Arxiv {
    pub fn new() -> Self {
        Arxiv::default()
    }

    /// Save the paper as a pdf from the information stored by the structure.
    pub async fn fetch_pdf(&self, out_path: &str) -> Result<()> {
        let body = reqwest::get(&self.pdf_url).await?.bytes().await?;
        let out_path = if out_path.ends_with(".pdf") {
            out_path.to_string()
        } else {
            format!("{out_path}.pdf")
        };
        let mut file = fs::File::create(out_path)?;
        file.write_all(&body)?;
        Ok(())
    }
}

/// A structure that stores the query information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ArxivQuery {
    pub base_url: String,
    pub search_query: String,
    pub id_list: String,
    pub start: Option<i32>,
    pub max_results: Option<i32>,
    pub sort_by: String,
    pub sort_order: String,
}

impl ArxivQuery {
    /// Generate a URL string.
    pub fn to_url(&self) -> String {
        let mut querys = Vec::new();
        if !self.search_query.is_empty() {
            querys.push(format!("search_query={}", self.search_query));
        }
        if !self.id_list.is_empty() {
            querys.push(format!("id_list={}", self.id_list));
        }
        if let Some(start) = self.start {
            querys.push(format!("start={start}"));
        }
        if let Some(max_results) = self.max_results {
            querys.push(format!("max_results={max_results}"));
        }
        if !self.sort_by.is_empty() {
            querys.push(format!("sortBy={}", self.sort_by));
        }
        if !self.sort_order.is_empty() {
            querys.push(format!("sortOrder={}", self.sort_order));
        }
        format!("{}{}", self.base_url, querys.join("&"))
    }
}

/// A builder of ArxivQuery
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ArxivQueryBuilder {
    pub base_url: String,
    pub search_query: String,
    pub id_list: String,
    pub start: Option<i32>,
    pub max_results: Option<i32>,
    pub sort_by: String,
    pub sort_order: String,
}

impl ArxivQueryBuilder {
    pub fn new() -> Self {
        ArxivQueryBuilder {
            base_url: "http://export.arxiv.org/api/query?".to_string(),
            ..ArxivQueryBuilder::default()
        }
    }
    /// Build ArxivQuery from ArxivQueryBuilder.
    pub fn build(&self) -> ArxivQuery {
        ArxivQuery {
            base_url: self.base_url.clone(),
            search_query: self.search_query.clone(),
            id_list: self.id_list.clone(),
            start: self.start,
            max_results: self.max_results,
            sort_by: self.sort_by.clone(),
            sort_order: self.sort_order.clone(),
        }
    }
    /// Store the argument value in search_query.
    pub fn search_query(&self, search_query: &str) -> Self {
        ArxivQueryBuilder {
            search_query: search_query.to_string(),
            ..self.clone()
        }
    }
    /// Store the argument value in id_list.
    pub fn id_list(&self, id_list: &str) -> Self {
        ArxivQueryBuilder {
            id_list: id_list.to_string(),
            ..self.clone()
        }
    }
    /// Store the argument value in start.
    pub fn start(&self, start: i32) -> Self {
        ArxivQueryBuilder {
            start: Some(start),
            ..self.clone()
        }
    }
    /// Store the argument value in max_results.
    pub fn max_results(&self, max_results: i32) -> Self {
        ArxivQueryBuilder {
            max_results: Some(max_results),
            ..self.clone()
        }
    }
    /// Store the argument value in sort_by.
    pub fn sort_by(&self, sort_by: &str) -> Self {
        ArxivQueryBuilder {
            sort_by: sort_by.to_string(),
            ..self.clone()
        }
    }
    /// Store the argument value in sort_order.
    pub fn sort_order(&self, sort_order: &str) -> Self {
        ArxivQueryBuilder {
            sort_order: sort_order.to_string(),
            ..self.clone()
        }
    }
}

/// Fetch the paper information using the arXiv API.
/// # Example
/// ```rust
/// use arxiv::{fetch_arxivs, query};
///
/// let query = query!(search_query = "cat:cs.CL");
/// // arxivs type is Vec<Arxiv>
/// let arxivs = fetch_arxivs(query).await?;
/// ```
pub async fn fetch_arxivs(query: ArxivQuery) -> Result<Vec<Arxiv>> {
    let body = reqwest::get(query.to_url()).await?.text().await?;
    let arxivs = parse_data(body)?;
    Ok(arxivs)
}

fn parse_data(body: String) -> Result<Vec<Arxiv>> {
    let mut parser = EventReader::from_str(&body);
    let mut arxiv = Arxiv::new();
    let mut arxivs = Vec::new();

    'outer: loop {
        match parser.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => match &name.local_name[..] {
                "entry" => {
                    arxiv = Arxiv::new();
                }
                "id" => {
                    if let XmlEvent::Characters(id) = parser.next()? {
                        arxiv.id = id;
                    }
                }
                "updated" => {
                    if let XmlEvent::Characters(updated) = parser.next()? {
                        arxiv.updated = updated
                    }
                }
                "published" => {
                    if let XmlEvent::Characters(published) = parser.next()? {
                        arxiv.published = published
                    }
                }
                "title" => {
                    if let XmlEvent::Characters(title) = parser.next()? {
                        arxiv.title = title
                    }
                }
                "summary" => {
                    if let XmlEvent::Characters(summary) = parser.next()? {
                        arxiv.summary = summary
                    }
                }
                "author" => {
                    parser.next()?;
                    parser.next()?;
                    if let XmlEvent::Characters(author) = parser.next()? {
                        arxiv.authors.push(author);
                    }
                }
                "primary_category" => {
                    if let Some(attribute) = attributes
                        .iter()
                        .find(|attr| attr.name.local_name == "term")
                    {
                        arxiv.primary_category = attribute.value.clone();
                        arxiv.primary_category_name = get_category_code_name(&attribute.value);
                    }
                }
                "category" => {
                    if let Some(attribute) = attributes
                        .iter()
                        .find(|attr| attr.name.local_name == "term")
                    {
                        arxiv.categories.push(attribute.value.clone());
                        arxiv
                            .category_names
                            .push(get_category_code_name(&attribute.value));
                    }
                }
                "link" => {
                    if attributes
                        .iter()
                        .any(|attr| attr.name.local_name == "title" && attr.value == "pdf")
                        && let Some(attribute) = attributes
                            .iter()
                            .find(|attr| attr.name.local_name == "href")
                    {
                        arxiv.pdf_url = format!(
                            "{}.pdf",
                            attribute.value.replacen("http", "https", 1).clone()
                        );
                    }

                    if attributes
                        .iter()
                        .any(|attr| attr.name.local_name == "type" && attr.value == "text/html")
                        && let Some(attribute) = attributes
                            .iter()
                            .find(|attr| attr.name.local_name == "href")
                    {
                        arxiv.html_url = attribute.value.replacen("http", "https", 1).clone();
                    }
                }
                "comment" => {
                    if let XmlEvent::Characters(comment) = parser.next()? {
                        arxiv.comment = Some(comment);
                    }
                }
                _ => (),
            },
            XmlEvent::EndElement { name } => match &name.local_name[..] {
                "entry" => {
                    arxivs.push(arxiv.clone());
                }
                "feed" => {
                    break 'outer;
                }
                _ => (),
            },
            _ => (),
        }
    }
    Ok(arxivs)
}

fn get_category_code_name(code: &str) -> String {
    match category_code_names.get(code) {
        Some(name) => name.to_string(),
        None => code.to_owned(),
    }
}

lazy_static::lazy_static! {
    static ref category_code_names: HashMap<String, &'static str> = hash_map![
        "cs.AI".to_owned() => "Artificial Intelligence",
        "cs.AR".to_owned() => "Hardware Architecture",
        "cs.CC".to_owned() => "Computational Complexity",
        "cs.CE".to_owned() => "Computational Engineering, Finance, and Science",
        "cs.CG".to_owned() => "Computational Geometry",
        "cs.CL".to_owned() => "Computation and Language",
        "cs.CR".to_owned() => "Cryptography and Security",
        "cs.CV".to_owned() => "Computer Vision and Pattern Recognition",
        "cs.CY".to_owned() => "Computers and Society",
        "cs.DB".to_owned() => "Databases",
        "cs.DC".to_owned() => "Distributed, Parallel, and Cluster Computing",
        "cs.DL".to_owned() => "Digital Libraries",
        "cs.DM".to_owned() => "Discrete Mathematics",
        "cs.DS".to_owned() => "Data Structures and Algorithms",
        "cs.ET".to_owned() => "Emerging Technologies",
        "cs.FL".to_owned() => "Formal Languages and Automata Theory",
        "cs.GL".to_owned() => "General Literature",
        "cs.GR".to_owned() => "Graphics",
        "cs.GT".to_owned() => "Computer Science and Game Theory",
        "cs.HC".to_owned() => "Human-Computer Interaction",
        "cs.IR".to_owned() => "Information Retrieval",
        "cs.IT".to_owned() => "Information Theory",
        "cs.LG".to_owned() => "Machine Learning",
        "cs.LO".to_owned() => "Logic in Computer Science",
        "cs.MA".to_owned() => "Multiagent Systems",
        "cs.MM".to_owned() => "Multimedia",
        "cs.MS".to_owned() => "Mathematical Software",
        "cs.NA".to_owned() => "Numerical Analysis",
        "cs.NE".to_owned() => "Neural and Evolutionary Computing",
        "cs.NI".to_owned() => "Networking and Internet Architecture",
        "cs.OH".to_owned() => "Other Computer Science",
        "cs.OS".to_owned() => "Operating Systems",
        "cs.PF".to_owned() => "Performance",
        "cs.PL".to_owned() => "Programming Languages",
        "cs.RO".to_owned() => "Robotics",
        "cs.SC".to_owned() => "Symbolic Computation",
        "cs.SD".to_owned() => "Sound",
        "cs.SE".to_owned() => "Software Engineering",
        "cs.SI".to_owned() => "Social and Information Networks",
        "cs.SY".to_owned() => "Systems and Control",
        "econ.EM".to_owned() => "Econometrics",
        "econ.GN".to_owned() => "General Economics",
        "econ.TH".to_owned() => "Theoretical Economics",
        "eess.AS".to_owned() => "Audio and Speech Processing",
        "eess.IV".to_owned() => "Image and Video Processing",
        "eess.SP".to_owned() => "Signal Processing",
        "eess.SY".to_owned() => "Systems and Control",
        "math.AC".to_owned() => "Commutative Algebra",
        "math.AG".to_owned() => "Algebraic Geometry",
        "math.AP".to_owned() => "Analysis of PDEs",
        "math.AT".to_owned() => "Algebraic Topology",
        "math.CA".to_owned() => "Classical Analysis and ODEs",
        "math.CO".to_owned() => "Combinatorics",
        "math.CT".to_owned() => "Category Theory",
        "math.CV".to_owned() => "Complex Variables",
        "math.DG".to_owned() => "Differential Geometry",
        "math.DS".to_owned() => "Dynamical Systems",
        "math.FA".to_owned() => "Functional Analysis",
        "math.GM".to_owned() => "General Mathematics",
        "math.GN".to_owned() => "General Topology",
        "math.GR".to_owned() => "Group Theory",
        "math.GT".to_owned() => "Geometric Topology",
        "math.HO".to_owned() => "History and Overview",
        "math.IT".to_owned() => "Information Theory",
        "math.KT".to_owned() => "K-Theory and Homology",
        "math.LO".to_owned() => "Logic",
        "math.MG".to_owned() => "Metric Geometry",
        "math.MP".to_owned() => "Mathematical Physics",
        "math.NA".to_owned() => "Numerical Analysis",
        "math.NT".to_owned() => "Number Theory",
        "math.OA".to_owned() => "Operator Algebras",
        "math.OC".to_owned() => "Optimization and Control",
        "math.PR".to_owned() => "Probability",
        "math.QA".to_owned() => "Quantum Algebra",
        "math.RA".to_owned() => "Rings and Algebras",
        "math.RT".to_owned() => "Representation Theory",
        "math.SG".to_owned() => "Symplectic Geometry",
        "math.SP".to_owned() => "Spectral Theory",
        "math.ST".to_owned() => "Statistics Theory",
        "astro-ph.CO".to_owned() => "Cosmology and Nongalactic Astrophysics",
        "astro-ph.EP".to_owned() => "Earth and Planetary Astrophysics",
        "astro-ph.GA".to_owned() => "Astrophysics of Galaxies",
        "astro-ph.HE".to_owned() => "High Energy Astrophysical Phenomena",
        "astro-ph.IM".to_owned() => "Instrumentation and Methods for Astrophysics",
        "astro-ph.SR".to_owned() => "Solar and Stellar Astrophysics",
        "cond-mat.dis-nn".to_owned() => "Disordered Systems and Neural Networks",
        "cond-mat.mes-hall".to_owned() => "Mesoscale and Nanoscale Physics",
        "cond-mat.mtrl-sci".to_owned() => "Materials Science",
        "cond-mat.other".to_owned() => "Other Condensed Matter",
        "cond-mat.quant-gas".to_owned() => "Quantum Gases",
        "cond-mat.soft".to_owned() => "Soft Condensed Matter",
        "cond-mat.stat-mech".to_owned() => "Statistical Mechanics",
        "cond-mat.str-el".to_owned() => "Strongly Correlated Electrons",
        "cond-mat.supr-con".to_owned() => "Superconductivity",
        "gr-qc".to_owned() => "General Relativity and Quantum Cosmology",
        "hep-ex".to_owned() => "High Energy Physics - Experiment",
        "hep-lat".to_owned() => "High Energy Physics - Lattice",
        "hep-ph".to_owned() => "High Energy Physics - Phenomenology",
        "hep-th".to_owned() => "High Energy Physics - Theory",
        "math-ph".to_owned() => "Mathematical Physics",
        "nlin.AO".to_owned() => "Adaptation and Self-Organizing Systems",
        "nlin.CD".to_owned() => "Chaotic Dynamics",
        "nlin.CG".to_owned() => "Cellular Automata and Lattice Gases",
        "nlin.PS".to_owned() => "Pattern Formation and Solitons",
        "nlin.SI".to_owned() => "Exactly Solvable and Integrable Systems",
        "nucl-ex".to_owned() => "Nuclear Experiment",
        "nucl-th".to_owned() => "Nuclear Theory",
        "physics.acc-ph".to_owned() => "Accelerator Physics",
        "physics.ao-ph".to_owned() => "Atmospheric and Oceanic Physics",
        "physics.app-ph".to_owned() => "Applied Physics",
        "physics.atm-clus".to_owned() => "Atomic and Molecular Clusters",
        "physics.atom-ph".to_owned() => "Atomic Physics",
        "physics.bio-ph".to_owned() => "Biological Physics",
        "physics.chem-ph".to_owned() => "Chemical Physics",
        "physics.class-ph".to_owned() => "Classical Physics",
        "physics.comp-ph".to_owned() => "Computational Physics",
        "physics.data-an".to_owned() => "Data Analysis, Statistics and Probability",
        "physics.ed-ph".to_owned() => "Physics Education",
        "physics.flu-dyn".to_owned() => "Fluid Dynamics",
        "physics.gen-ph".to_owned() => "General Physics",
        "physics.geo-ph".to_owned() => "Geophysics",
        "physics.hist-ph".to_owned() => "History and Philosophy of Physics",
        "physics.ins-det".to_owned() => "Instrumentation and Detectors",
        "physics.med-ph".to_owned() => "Medical Physics",
        "physics.optics".to_owned() => "Optics",
        "physics.plasm-ph".to_owned() => "Plasma Physics",
        "physics.pop-ph".to_owned() => "Popular Physics",
        "physics.soc-ph".to_owned() => "Physics and Society",
        "physics.space-ph".to_owned() => "Space Physics",
        "quant-ph".to_owned() => "Quantum Physics",
        "q-bio.BM".to_owned() => "Biomolecules",
        "q-bio.CB".to_owned() => "Cell Behavior",
        "q-bio.GN".to_owned() => "Genomics",
        "q-bio.MN".to_owned() => "Molecular Networks",
        "q-bio.NC".to_owned() => "Neurons and Cognition",
        "q-bio.OT".to_owned() => "Other Quantitative Biology",
        "q-bio.PE".to_owned() => "Populations and Evolution",
        "q-bio.QM".to_owned() => "Quantitative Methods",
        "q-bio.SC".to_owned() => "Subcellular Processes",
        "q-bio.TO".to_owned() => "Tissues and Organs",
        "q-fin.CP".to_owned() => "Computational Finance",
        "q-fin.EC".to_owned() => "Economics",
        "q-fin.GN".to_owned() => "General Finance",
        "q-fin.MF".to_owned() => "Mathematical Finance",
        "q-fin.PM".to_owned() => "Portfolio Management",
        "q-fin.PR".to_owned() => "Pricing of Securities",
        "q-fin.RM".to_owned() => "Risk Management",
        "q-fin.ST".to_owned() => "Statistical Finance",
        "q-fin.TR".to_owned() => "Trading and Market Microstructure",
        "stat.AP".to_owned() => "Applications",
        "stat.CO".to_owned() => "Computation",
        "stat.ME".to_owned() => "Methodology",
        "stat.ML".to_owned() => "Machine Learning",
        "stat.OT".to_owned() => "Other Statistics",
        "stat.TH".to_owned() => "Statistics Theory",
    ];
}
