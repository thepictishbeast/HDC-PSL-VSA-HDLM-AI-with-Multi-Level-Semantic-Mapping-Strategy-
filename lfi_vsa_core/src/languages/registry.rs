// ============================================================
// LFI Language Registry — Language-Specific Knowledge
// Section 1.III: Multi-Level Semantic Mapping
//
// Maps specific programming languages (Rust, Go, SQL, etc.)
// to their supported paradigms, platforms, and constructs.
// This enables the LFI agent to select the correct language
// for a given task and platform.
// ============================================================

use crate::languages::constructs::{Paradigm, PlatformTarget, UniversalConstruct};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Unique identifier for a programming language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageId {
    Rust,
    C,
    Cpp,
    Go,
    Java,
    Kotlin,
    Swift,
    Csharp,
    VisualBasic,
    Php,
    JavaScript,
    TypeScript,
    Python,
    Ruby,
    Elixir,
    Erlang,
    Haskell,
    Assembly,
    WebAssembly,
    Verilog,
    Sql,
    Html,
    Css,
    Shell,
    Dart,
    Scala,
}

/// Language-specific metadata and capability mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetadata {
    pub id: LanguageId,
    pub name: String,
    pub paradigms: Vec<Paradigm>,
    pub platforms: Vec<PlatformTarget>,
    pub primary_constructs: Vec<UniversalConstruct>,
}

/// Registry of all known programming languages and their capabilities.
pub struct LanguageRegistry {
    languages: HashMap<LanguageId, LanguageMetadata>,
}

impl LanguageRegistry {
    /// Create a new registry populated with core language knowledge.
    pub fn new() -> Self {
        debuglog!("LanguageRegistry::new: Initializing language knowledge");
        let mut languages = HashMap::new();

        // ---- Systems Languages ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Rust,
            name: "Rust".to_string(),
            paradigms: vec![Paradigm::Systems, Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Embedded, PlatformTarget::Web],
            primary_constructs: vec![UniversalConstruct::OwnershipBorrowing, UniversalConstruct::PatternMatch, UniversalConstruct::AsyncAwait],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Go,
            name: "Go".to_string(),
            paradigms: vec![Paradigm::Procedural, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::Channel, UniversalConstruct::ThreadSpawn, UniversalConstruct::ErrorHandling],
        });

        // ---- Web & Scripting ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::TypeScript,
            name: "TypeScript".to_string(),
            paradigms: vec![Paradigm::Functional, Paradigm::ObjectOriented, Paradigm::Reactive],
            platforms: vec![PlatformTarget::Web, PlatformTarget::Cloud, PlatformTarget::CrossPlatform],
            primary_constructs: vec![UniversalConstruct::AsyncAwait, UniversalConstruct::UIComponent, UniversalConstruct::GenericType],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Php,
            name: "PHP".to_string(),
            paradigms: vec![Paradigm::Procedural, Paradigm::ObjectOriented, Paradigm::Scripting],
            platforms: vec![PlatformTarget::Web, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::APIEndpoint, UniversalConstruct::DatabaseQuery, UniversalConstruct::Template],
        });

        // ---- JVM & Mobile ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Kotlin,
            name: "Kotlin".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Android, PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS],
            primary_constructs: vec![UniversalConstruct::AsyncAwait, UniversalConstruct::PatternMatch, UniversalConstruct::UIComponent],
        });

        // ---- Scripting & Dynamic ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Python,
            name: "Python".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Scripting],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Cloud, PlatformTarget::CrossPlatform],
            primary_constructs: vec![UniversalConstruct::Lambda, UniversalConstruct::HigherOrderFunction, UniversalConstruct::GarbageCollection],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::JavaScript,
            name: "JavaScript".to_string(),
            paradigms: vec![Paradigm::Functional, Paradigm::ObjectOriented, Paradigm::Reactive, Paradigm::Scripting],
            platforms: vec![PlatformTarget::Web, PlatformTarget::Cloud, PlatformTarget::CrossPlatform],
            primary_constructs: vec![UniversalConstruct::AsyncAwait, UniversalConstruct::Lambda, UniversalConstruct::UIComponent],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Ruby,
            name: "Ruby".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Scripting],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::MacOS, PlatformTarget::Web, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::Lambda, UniversalConstruct::HigherOrderFunction, UniversalConstruct::GarbageCollection],
        });

        // ---- JVM & Mobile (additional) ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Java,
            name: "Java".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Android, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::ClassDefinition, UniversalConstruct::Inheritance, UniversalConstruct::GarbageCollection],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Swift,
            name: "Swift".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::MacOS, PlatformTarget::IOS, PlatformTarget::Linux],
            primary_constructs: vec![UniversalConstruct::PatternMatch, UniversalConstruct::ReferenceCounting, UniversalConstruct::AsyncAwait],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Csharp,
            name: "CSharp".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Windows, PlatformTarget::Linux, PlatformTarget::MacOS, PlatformTarget::Web, PlatformTarget::CrossPlatform],
            primary_constructs: vec![UniversalConstruct::AsyncAwait, UniversalConstruct::ClassDefinition, UniversalConstruct::GarbageCollection],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Scala,
            name: "Scala".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::PatternMatch, UniversalConstruct::HigherOrderFunction, UniversalConstruct::Actor],
        });

        // ---- Systems (additional) ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::C,
            name: "C".to_string(),
            paradigms: vec![Paradigm::Procedural, Paradigm::Systems],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Embedded],
            primary_constructs: vec![UniversalConstruct::PointerReference, UniversalConstruct::ManualMemory, UniversalConstruct::StackAllocation],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Cpp,
            name: "Cpp".to_string(),
            paradigms: vec![Paradigm::Systems, Paradigm::ObjectOriented, Paradigm::Functional],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Embedded],
            primary_constructs: vec![UniversalConstruct::ClassDefinition, UniversalConstruct::GenericType, UniversalConstruct::ManualMemory],
        });

        // ---- Functional ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Elixir,
            name: "Elixir".to_string(),
            paradigms: vec![Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::PatternMatch, UniversalConstruct::Actor, UniversalConstruct::Channel],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Haskell,
            name: "Haskell".to_string(),
            paradigms: vec![Paradigm::Functional],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS],
            primary_constructs: vec![UniversalConstruct::PatternMatch, UniversalConstruct::HigherOrderFunction, UniversalConstruct::TailCall],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Erlang,
            name: "Erlang".to_string(),
            paradigms: vec![Paradigm::Functional, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::Actor, UniversalConstruct::PatternMatch, UniversalConstruct::Channel],
        });

        // ---- Query & Markup ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Sql,
            name: "SQL".to_string(),
            paradigms: vec![Paradigm::Declarative],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Cloud],
            primary_constructs: vec![UniversalConstruct::DatabaseQuery, UniversalConstruct::Conditional, UniversalConstruct::ArrayType],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Shell,
            name: "Shell".to_string(),
            paradigms: vec![Paradigm::Scripting, Paradigm::Procedural],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::MacOS],
            primary_constructs: vec![UniversalConstruct::SystemCall, UniversalConstruct::FileIO, UniversalConstruct::Conditional],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Dart,
            name: "Dart".to_string(),
            paradigms: vec![Paradigm::ObjectOriented, Paradigm::Reactive],
            platforms: vec![PlatformTarget::Android, PlatformTarget::IOS, PlatformTarget::Web, PlatformTarget::CrossPlatform],
            primary_constructs: vec![UniversalConstruct::AsyncAwait, UniversalConstruct::UIComponent, UniversalConstruct::StateManagement],
        });

        // ---- Hardware & Low-Level ----
        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Verilog,
            name: "Verilog".to_string(),
            paradigms: vec![Paradigm::HardwareDescription, Paradigm::Concurrent],
            platforms: vec![PlatformTarget::FPGA],
            primary_constructs: vec![UniversalConstruct::HdlRegister, UniversalConstruct::HdlWire, UniversalConstruct::HdlClockDomain],
        });

        Self::register(&mut languages, LanguageMetadata {
            id: LanguageId::Assembly,
            name: "Assembly".to_string(),
            paradigms: vec![Paradigm::LowLevel, Paradigm::Systems],
            platforms: vec![PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS, PlatformTarget::Embedded],
            primary_constructs: vec![UniversalConstruct::Jump, UniversalConstruct::ManualMemory, UniversalConstruct::StackAllocation],
        });

        Self { languages }
    }

    fn register(map: &mut HashMap<LanguageId, LanguageMetadata>, meta: LanguageMetadata) {
        debuglog!("LanguageRegistry::register: {}", meta.name);
        map.insert(meta.id.clone(), meta);
    }

    /// Retrieve metadata for a specific language.
    pub fn get_language(&self, id: &LanguageId) -> Option<&LanguageMetadata> {
        debuglog!("LanguageRegistry::get_language: {:?}", id);
        self.languages.get(id)
    }

    /// Find languages that support a specific paradigm.
    pub fn find_by_paradigm(&self, paradigm: Paradigm) -> Vec<&LanguageMetadata> {
        debuglog!("LanguageRegistry::find_by_paradigm: {:?}", paradigm);
        self.languages.values()
            .filter(|m| m.paradigms.contains(&paradigm))
            .collect()
    }

    /// Find languages that support a specific platform.
    pub fn find_by_platform(&self, platform: PlatformTarget) -> Vec<&LanguageMetadata> {
        debuglog!("LanguageRegistry::find_by_platform: {:?}", platform);
        self.languages.values()
            .filter(|m| m.platforms.contains(&platform))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = LanguageRegistry::new();
        assert!(registry.get_language(&LanguageId::Rust).is_some());
        assert!(registry.get_language(&LanguageId::Kotlin).is_some());
    }

    #[test]
    fn test_find_by_paradigm() {
        let registry = LanguageRegistry::new();
        let concurrent = registry.find_by_paradigm(Paradigm::Concurrent);
        // Rust, Go, Kotlin, Verilog, Java, Swift, Csharp, Scala, Elixir, Erlang
        assert!(concurrent.len() >= 4);
    }

    #[test]
    fn test_find_by_platform() {
        let registry = LanguageRegistry::new();
        let fpga = registry.find_by_platform(PlatformTarget::FPGA);
        assert_eq!(fpga.len(), 1);
        assert_eq!(fpga[0].id, LanguageId::Verilog);
    }

    #[test]
    fn test_all_languages_have_paradigms() {
        let registry = LanguageRegistry::new();
        for lang in registry.languages.values() {
            assert!(!lang.paradigms.is_empty(),
                "Language {:?} should have at least one paradigm", lang.id);
        }
    }

    #[test]
    fn test_rust_has_expected_properties() {
        let registry = LanguageRegistry::new();
        let rust = registry.get_language(&LanguageId::Rust).expect("Rust should exist");
        assert!(rust.paradigms.contains(&Paradigm::Systems));
        assert!(rust.paradigms.contains(&Paradigm::Concurrent));
    }

    #[test]
    fn test_language_id_serialization() {
        let id = LanguageId::Rust;
        let json = serde_json::to_string(&id).unwrap();
        let recovered: LanguageId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, LanguageId::Rust);
    }

    #[test]
    fn test_find_functional_languages() {
        let registry = LanguageRegistry::new();
        let functional = registry.find_by_paradigm(Paradigm::Functional);
        assert!(functional.len() >= 2, "Should find multiple functional languages: {}", functional.len());
    }

    // ============================================================
    // Stress / invariant tests for LanguageRegistry
    // ============================================================

    /// INVARIANT: every registered language has a non-empty name,
    /// at least one paradigm, at least one platform, and at least
    /// one primary construct.
    #[test]
    fn invariant_every_language_has_minimal_metadata() {
        let registry = LanguageRegistry::new();
        for lang in registry.languages.values() {
            assert!(!lang.name.is_empty(),
                "{:?} has empty name", lang.id);
            assert!(!lang.paradigms.is_empty(),
                "{:?} has no paradigms", lang.id);
            assert!(!lang.platforms.is_empty(),
                "{:?} has no platforms", lang.id);
            assert!(!lang.primary_constructs.is_empty(),
                "{:?} has no primary constructs", lang.id);
        }
    }

    /// INVARIANT: LanguageRegistry::new() is deterministic w.r.t. the
    /// set of language IDs (two instances register the same languages).
    #[test]
    fn invariant_new_is_deterministic() {
        let r1 = LanguageRegistry::new();
        let r2 = LanguageRegistry::new();
        let ids1: std::collections::HashSet<_> = r1.languages.keys().collect();
        let ids2: std::collections::HashSet<_> = r2.languages.keys().collect();
        assert_eq!(ids1, ids2, "registry set should be deterministic");
    }

    /// INVARIANT: get_language returns None for every id NOT registered.
    /// (Currently registry is maximally populated, so this checks that
    /// lookup returns Some for everything in .languages.)
    #[test]
    fn invariant_get_language_matches_internal_set() {
        let registry = LanguageRegistry::new();
        for id in registry.languages.keys().cloned().collect::<Vec<_>>() {
            assert!(registry.get_language(&id).is_some(),
                "failed lookup for registered {:?}", id);
        }
    }

    /// INVARIANT: find_by_paradigm ⊆ all languages, and every returned
    /// language actually lists that paradigm.
    #[test]
    fn invariant_find_by_paradigm_self_consistent() {
        let registry = LanguageRegistry::new();
        let paradigms = [
            Paradigm::Systems, Paradigm::Functional, Paradigm::Concurrent,
            Paradigm::Procedural, Paradigm::ObjectOriented, Paradigm::Declarative,
            Paradigm::Scripting, Paradigm::Reactive, Paradigm::HardwareDescription,
            Paradigm::LowLevel,
        ];
        for p in paradigms {
            for m in registry.find_by_paradigm(p.clone()) {
                assert!(m.paradigms.contains(&p),
                    "{:?} returned for {:?} but doesn't list it", m.id, p);
            }
        }
    }

    /// INVARIANT: find_by_platform returns only languages that list the platform.
    #[test]
    fn invariant_find_by_platform_self_consistent() {
        let registry = LanguageRegistry::new();
        let platforms = [
            PlatformTarget::Linux, PlatformTarget::Windows, PlatformTarget::MacOS,
            PlatformTarget::Web, PlatformTarget::Cloud, PlatformTarget::FPGA,
            PlatformTarget::Android, PlatformTarget::IOS, PlatformTarget::Embedded,
            PlatformTarget::CrossPlatform,
        ];
        for pl in platforms {
            for m in registry.find_by_platform(pl.clone()) {
                assert!(m.platforms.contains(&pl),
                    "{:?} returned for {:?} but doesn't list it", m.id, pl);
            }
        }
    }

    /// INVARIANT: LanguageId serialization round-trips for every registered id.
    #[test]
    fn invariant_all_language_ids_serde_roundtrip() {
        let registry = LanguageRegistry::new();
        for id in registry.languages.keys() {
            let json = serde_json::to_string(id).unwrap();
            let recovered: LanguageId = serde_json::from_str(&json).unwrap();
            assert_eq!(&recovered, id);
        }
    }
}
