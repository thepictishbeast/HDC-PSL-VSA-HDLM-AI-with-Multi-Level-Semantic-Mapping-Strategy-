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
}
