use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use regex::Regex;

pub struct Linguist {
    lemmatizer: HashMap<String, String>,
    stopwords: HashSet<String>,
}

impl Linguist {
    pub fn new() -> Self {
        Linguist {
            lemmatizer: HashMap::new(),
            stopwords: HashSet::new(),
        }
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
            // Column 1 is the Source (Inflected form)
            let lemma = parts[0].to_string();
            let inflected = parts[1].to_string();
            
            // Map the inflected word to its lemma
            self.lemmatizer.insert(inflected, lemma);
        }
    }
    Ok(())
}

    pub fn process(&self, text: &str) -> Vec<String> {
        // 1. Casefolding
        let lowercased = text.to_lowercase();

        // 2. Tokenization (split by non-alphanumeric characters)
        let re = Regex::new(r"[^a-z0-9]+").unwrap();
        let tokens: Vec<&str> = re.split(&lowercased).collect();

        tokens.into_iter()
            .filter(|t| !t.is_empty())
            .filter_map(|token| {
                // 3. Stopword Removal
                if self.stopwords.contains(token) {
                    return None;
                }

                // 4. Lemmatization
                // If the token exists in our map, replace it with the lemma.
                // Otherwise, keep the original token.
                let lemma = self.lemmatizer.get(token).map_or(token, |v| v);
                
                Some(lemma.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing() {
        let mut linguist = Linguist::new();
        // Mock lemmatization data
        linguist.lemmatizer.insert("running".to_string(), "run".to_string());
        linguist.lemmatizer.insert("cats".to_string(), "cat".to_string());
        
        // Mock stopwords
        linguist.stopwords.insert("the".to_string());
        linguist.stopwords.insert("are".to_string());

        let text = "The cats are running fast!";
        let tokens = linguist.process(text);
        
        // "the", "are" are stopwords.
        // "cats" -> "cat"
        // "running" -> "run"
        // "fast" -> "fast"
        assert_eq!(tokens, vec!["cat", "run", "fast"]);
    }
}
