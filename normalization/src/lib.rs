use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use regex::Regex;

pub struct Normalizer {
    lemmatizer: HashMap<String, String>,
    stopwords: HashSet<String>,
    whitelist: HashSet<String>,
    token_splitter: Regex,
}

impl Normalizer {
    pub fn new() -> Self {
        Normalizer {
            lemmatizer: HashMap::new(),
            stopwords: HashSet::new(),
            whitelist: HashSet::new(),
            token_splitter: Regex::new(r"[^a-z0-9]+").unwrap(),
        }
    }

    pub fn load_whitelist<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let word = line?.trim().to_string();
            if !word.is_empty() {
                self.whitelist.insert(word.to_lowercase());
            }
        }
        Ok(())
    }

    pub fn load_stopwords<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let word = line?.trim().to_string();
            if !word.is_empty() {
                self.stopwords.insert(word);
            }
        }
        Ok(())
    }

pub fn load_lemmatization_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            // Column 0 is the Target (Lemma)
            // Column 1+ are the Source forms (Inflected forms)
            let lemma = parts[0].to_string();
            
            // Map each inflected form to its lemma
            for &inflected_form in &parts[1..] {
                self.lemmatizer.insert(inflected_form.to_string(), lemma.clone());
            }
        }
    }
    Ok(())
}

    pub fn process(&self, text: &str) -> Vec<String> {
        let lowercased = text.to_lowercase();
        let mut tokens = Vec::new();

        for raw_token in lowercased.split_whitespace() {
            if let Some(whitelisted) = self.find_whitelisted_token(raw_token) {
                tokens.push(whitelisted);
            } else {
                // Fallback: standard tokenization
                tokens.extend(
                    self.token_splitter
                        .split(raw_token)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                );
            }
        }

        // Filter stopwords and apply lemmatization
        tokens
            .into_iter()
            .filter(|t| !self.stopwords.contains(t))
            .map(|token| self.lemmatizer.get(&token).cloned().unwrap_or(token))
            .collect()
    }

    fn find_whitelisted_token(&self, token: &str) -> Option<String> {
        // Start with the most conservative trim (keep +, #, .)
        let base = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '+' && c != '#' && c != '.');
        
        if base.is_empty() {
            return None;
        }

        // Try progressively more aggressive trimming
        if self.whitelist.contains(base) {
            return Some(base.to_string());
        }
        
        let no_trailing = base.trim_end_matches('.');
        if !no_trailing.is_empty() && self.whitelist.contains(no_trailing) {
            return Some(no_trailing.to_string());
        }
        
        let no_leading = base.trim_start_matches('.');
        if !no_leading.is_empty() && self.whitelist.contains(no_leading) {
            return Some(no_leading.to_string());
        }
        
        let no_dots = base.trim_matches('.');
        if !no_dots.is_empty() && self.whitelist.contains(no_dots) {
            return Some(no_dots.to_string());
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing() {
        let mut normalizer = Normalizer::new();
        // Mock lemmatization data
        normalizer.lemmatizer.insert("running".to_string(), "run".to_string());
        normalizer.lemmatizer.insert("cats".to_string(), "cat".to_string());
        
        // Mock stopwords
        normalizer.stopwords.insert("the".to_string());
        normalizer.stopwords.insert("are".to_string());

        // Mock whitelist
        normalizer.whitelist.insert("c++".to_string());
        normalizer.whitelist.insert("c#".to_string());
        normalizer.whitelist.insert(".net".to_string());
        normalizer.whitelist.insert("node.js".to_string());

        // Test 1: Basic processing
        let text = "The cats are running fast!";
        let tokens = normalizer.process(text);
        
        // "the", "are" are stopwords.
        // "cats" -> "cat"
        // "running" -> "run"
        // "fast" -> "fast"
        assert_eq!(tokens, vec!["cat", "run", "fast"]);

        // Test 2: Whitelist processing
        let text_whitelist = "I love c++ and c# but also node.js and .net framework.";
        let tokens_whitelist = normalizer.process(text_whitelist);
        
        let expected_whitelist = vec![
            "i", "love", "c++", "and", "c#", "but", "also", "node.js", "and", ".net", "framework"
        ];
        assert_eq!(tokens_whitelist, expected_whitelist);

        // Test 3: Whitelist mixed with punctuation
        let text_punct = "The language is (c++), or [c++]!";
        let tokens_punct = normalizer.process(text_punct);
        // "the" is stopword
        assert_eq!(tokens_punct, vec!["language", "is", "c++", "or", "c++"]);
    }
}
