use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub(crate) struct Changes {
    pub(crate) changed: Vec<(String, ObjectType)>, // hash change
    pub(crate) removed: Vec<(String, ObjectType)>, // symbol removed
    pub(crate) added: Vec<(String, ObjectType)>,   // new symbol
}

fn update_log(updates: &Vec<(String, ObjectType)>, log: &mut String, prefix: &str) {
    for update in updates {
        let (symbol, type_of) = update;
        let line = format!("{} {} ({})", prefix, symbol, type_of);
        log.push_str(&line);
        log.push('\n');
    }
}

impl Changes {
    pub(crate) fn new() -> Self {
        Self {
            changed: vec![],
            removed: vec![],
            added: vec![],
        }
    }

    pub(crate) fn generate_change_log(&self) -> String {
        let mut change_log = String::new();

        if self.added.len() > 0 {
            change_log.push_str("Added:\n");
            update_log(&self.added, &mut change_log, "    +");

        }


        if self.changed.len() > 0 {
            change_log.push_str("Changes:\n");
            update_log(&self.changed, &mut change_log, "    *");
        }

        if self.removed.len() > 0 {
            change_log.push_str("Removed:\n");
            update_log(&self.removed, &mut change_log, "    -");
        }

        change_log
    }

    pub(crate) fn has_changes(&self) -> bool {
        !self.changed.is_empty() || !self.removed.is_empty() || !self.added.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ObjectType {
    Function,
    Struct,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Function => write!(f, "function"),
            ObjectType::Struct => write!(f, "struct"),
        }
    }
}

impl Serialize for ObjectType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ObjectType::Function => serializer.serialize_str("function"),
            ObjectType::Struct => serializer.serialize_str("struct"),
        }
    }
}

impl<'de> Deserialize<'de> for ObjectType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "function" => Ok(ObjectType::Function),
            "struct" => Ok(ObjectType::Struct),
            _ => Err(serde::de::Error::custom("invalid ObjectType")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GenericSymbol {
    pub(crate) name: String,
    pub(crate) hash: String,
    pub(crate) type_of: ObjectType,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SymbolTable {
    pub(crate) symbols_map: HashMap<String, (String, ObjectType)>, // name -> hash
    pub(crate) major_version: u32,
    pub(crate) minor_version: u32,
    pub(crate) patch_version: u32,
}

impl SymbolTable {
    pub(crate) fn new(symbols: Vec<GenericSymbol>) -> Self {
        let mut symbols_map = HashMap::new();
        for symbol in symbols {
            symbols_map.insert(symbol.name, (symbol.hash, symbol.type_of));
        }

        Self {
            symbols_map,
            major_version: 0,
            minor_version: 0,
            patch_version: 0,
        }
    }

    pub(crate) fn compare_and_swap(
        &mut self,
        new_symbols: Vec<GenericSymbol>,
        force_major: bool,
    ) -> Changes {
        let mut symbol_changes = Changes::new();
        let old_symbols = &self.symbols_map;

        let old_symbols_names = old_symbols.keys().collect::<Vec<&String>>();
        let new_symbol_names = new_symbols
            .iter()
            .map(|x| x.name.clone())
            .collect::<Vec<String>>();

        for symb in &new_symbols {
            let (name, hash, type_of) = (&symb.name, &symb.hash, &symb.type_of);
            let fetch = old_symbols.get(name);
            if fetch.is_none() {
                symbol_changes.added.push((name.to_string(), *type_of));
            } else {
                let (old_hash, _old_type) = fetch.unwrap();
                if old_hash != hash {
                    symbol_changes.changed.push((name.to_string(), *type_of));
                }
            }
        }

        for symb in old_symbols_names {
            if !new_symbol_names.contains(symb) {
                let (_, type_of) = old_symbols.get(symb).unwrap();
                symbol_changes.removed.push((symb.to_string(), *type_of));
            }
        }

        // Update the symbol table with new symbols
        self.symbols_map.clear();
        for symbol in new_symbols {
            self.symbols_map
                .insert(symbol.name, (symbol.hash, symbol.type_of));
        }

        let changed_symbols = symbol_changes.changed.len();
        let removed_symbols = symbol_changes.removed.len();
        let added_symbols = symbol_changes.added.len();

        if force_major {
            // Force major bump
            self.major_version += 1;
            self.minor_version = 0;
            self.patch_version = 0;
        } else {
            // only hash changes = ++ patch (and nothing else!)
            if changed_symbols > 0 && removed_symbols == 0 && added_symbols == 0 {
                self.patch_version += 1;
            }
            // add or drop symbols = ++ minor
            if removed_symbols > 0 || added_symbols > 0 {
                self.minor_version += 1;
                self.patch_version = 0;
            }
        }

        symbol_changes
    }

    pub(crate) fn version_string(&self) -> String {
        format!(
            "{}.{}.{}",
            self.major_version, self.minor_version, self.patch_version
        )
    }
}
