// ============================================================
// Universal Programming Constructs — Cross-Language Taxonomy
//
// Every programming language is a composition of universal
// constructs: loops, branches, functions, types, etc.
// By encoding these constructs as orthogonal hypervectors,
// the LFI agent can reason about ANY language as a specific
// combination of these primitives.
//
// A "for loop" in Python and a "for loop" in Rust share the
// same base vector — the language-specific syntax is decorative
// (Tier 2), while the control flow semantics are forensic (Tier 1).
// ============================================================

use serde::{Serialize, Deserialize};

/// Programming paradigm classification.
/// Languages may belong to multiple paradigms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Paradigm {
    /// Object-oriented (Java, C#, Python, Ruby, Kotlin, Swift)
    ObjectOriented,
    /// Functional (Elixir, Haskell, OCaml, Clojure, F#, Erlang)
    Functional,
    /// Procedural / Imperative (C, Go, Fortran, Pascal)
    Procedural,
    /// Systems programming (Rust, C, C++, Assembly)
    Systems,
    /// Logic programming (Prolog, Datalog)
    Logic,
    /// Concurrent / Actor-based (Erlang, Elixir, Go, Akka)
    Concurrent,
    /// Declarative (SQL, HTML, CSS, Terraform, YAML)
    Declarative,
    /// Hardware description (Verilog, VHDL, SystemVerilog)
    HardwareDescription,
    /// Scripting (Python, Ruby, Perl, Lua, Bash)
    Scripting,
    /// Reactive / Event-driven (React, Angular, RxJS)
    Reactive,
    /// Low-level / Bare metal (Assembly, WebAssembly)
    LowLevel,
}

/// Target platform for code generation and architecture knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformTarget {
    /// Linux (x86_64, ARM64, RISC-V)
    Linux,
    /// Windows (x86_64, ARM64)
    Windows,
    /// macOS / Darwin (Apple Silicon, x86_64)
    MacOS,
    /// iOS (ARM64)
    IOS,
    /// Android (ARM64, x86_64 emulator)
    Android,
    /// Web / Browser (WASM, JavaScript)
    Web,
    /// Embedded / RTOS (ARM Cortex-M, RISC-V, AVR)
    Embedded,
    /// FPGA / ASIC (Verilog, VHDL synthesis targets)
    FPGA,
    /// Cloud / Serverless (AWS Lambda, GCP Functions, Azure)
    Cloud,
    /// Cross-platform (Electron, Flutter, React Native, .NET MAUI)
    CrossPlatform,
}

/// Universal programming constructs that exist across languages.
/// Each variant represents a semantic concept, not syntax.
/// The LFI codebook maps each construct to a unique hypervector.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniversalConstruct {
    // ---- Control Flow ----
    /// Sequential block (begin/end, {}, do/end, indentation)
    Block,
    /// Conditional branch (if/else, match/case, switch, when, cond)
    Conditional,
    /// Bounded iteration (for, foreach, for..in, Range-based)
    ForLoop,
    /// Unbounded iteration (while, loop, repeat/until)
    WhileLoop,
    /// Pattern matching (match, case, when — Rust, Elixir, Haskell, Kotlin)
    PatternMatch,
    /// Exception/error handling (try/catch, rescue, Result, Option)
    ErrorHandling,
    /// Early return / break / continue
    FlowControl,
    /// Coroutine / async-await / generator / yield
    AsyncAwait,
    /// Goto / labeled break (C, Assembly, some modern uses)
    Jump,

    // ---- Data & Types ----
    /// Variable binding (let, var, val, const, auto, dim)
    VariableBinding,
    /// Type declaration (type, typedef, alias, newtype)
    TypeDeclaration,
    /// Struct / record / data class / named tuple
    StructDefinition,
    /// Enum / union / algebraic data type / sealed class
    EnumDefinition,
    /// Array / list / vector / sequence
    ArrayType,
    /// Map / dictionary / hash / associative array
    MapType,
    /// Tuple / pair / product type
    TupleType,
    /// Optional / nullable / Maybe / Option
    OptionalType,
    /// Generic / template / parameterized type
    GenericType,
    /// Pointer / reference / borrow
    PointerReference,
    /// String type and operations
    StringType,

    // ---- Functions & Callables ----
    /// Function definition (fn, def, func, function, sub, method)
    FunctionDefinition,
    /// Lambda / closure / anonymous function / block
    Lambda,
    /// Function call / invocation
    FunctionCall,
    /// Higher-order function (map, filter, reduce, fold)
    HigherOrderFunction,
    /// Recursion (direct and mutual)
    Recursion,
    /// Tail call optimization
    TailCall,

    // ---- OOP Constructs ----
    /// Class definition
    ClassDefinition,
    /// Interface / protocol / trait / abstract class
    InterfaceDefinition,
    /// Inheritance / extension / subclassing
    Inheritance,
    /// Method dispatch / virtual method / dynamic dispatch
    MethodDispatch,
    /// Encapsulation (public/private/protected/internal)
    AccessControl,
    /// Constructor / initializer / __init__ / new
    Constructor,
    /// Destructor / finalizer / drop / deinit / __del__
    Destructor,

    // ---- Concurrency ----
    /// Thread / goroutine / fiber / green thread
    ThreadSpawn,
    /// Mutex / lock / synchronized / critical section
    MutexLock,
    /// Channel / message passing / mailbox (Go, Erlang, Rust)
    Channel,
    /// Atomic operation / compare-and-swap
    AtomicOp,
    /// Actor model (Erlang, Elixir, Akka)
    Actor,
    /// Future / promise / Task / deferred
    Future,

    // ---- Memory ----
    /// Heap allocation (malloc, new, Box, alloc)
    HeapAllocation,
    /// Stack allocation / value types
    StackAllocation,
    /// Garbage collection (Java, Go, Python, C#, JS)
    GarbageCollection,
    /// Manual memory management (C, C++)
    ManualMemory,
    /// Ownership / borrowing / lifetime (Rust)
    OwnershipBorrowing,
    /// Reference counting (Swift ARC, Rust Rc/Arc, ObjC)
    ReferenceCounting,

    // ---- Modules & Organization ----
    /// Module / package / namespace / crate
    ModuleDefinition,
    /// Import / use / require / include / using
    Import,
    /// Export / pub / public / module_exports
    Export,
    /// Macro / metaprogramming / compile-time code generation
    Macro,
    /// Annotation / attribute / decorator / pragma
    Annotation,

    // ---- I/O & System ----
    /// File I/O (read, write, open, close)
    FileIO,
    /// Network I/O (socket, HTTP, TCP/UDP)
    NetworkIO,
    /// Database query (SQL, ORM, query builder)
    DatabaseQuery,
    /// System call / FFI / extern / P/Invoke / JNI
    SystemCall,
    /// Serialization / deserialization (JSON, protobuf, XML)
    Serialization,

    // ---- Web & UI ----
    /// Component (React, Angular, Vue, Svelte, SwiftUI)
    UIComponent,
    /// Routing (web routes, navigation, URL mapping)
    Routing,
    /// State management (Redux, MobX, Provider, BLoC)
    StateManagement,
    /// Template / JSX / HTML template / Blade / ERB
    Template,
    /// CSS / styling / layout
    Styling,
    /// API endpoint (REST, GraphQL, gRPC)
    APIEndpoint,
    /// Middleware / interceptor / filter
    Middleware,

    // ---- Hardware Description ----
    /// Register / flip-flop (Verilog reg, VHDL signal)
    HdlRegister,
    /// Wire / combinational logic
    HdlWire,
    /// Clock domain / synchronous block (always @posedge)
    HdlClockDomain,
    /// Module instantiation (Verilog module, VHDL entity)
    HdlModuleInstance,
    /// Testbench / simulation
    HdlTestbench,

    // ---- Security ----
    /// Authentication (login, token, OAuth, SAML)
    Authentication,
    /// Authorization (RBAC, ABAC, ACL, permissions)
    Authorization,
    /// Encryption (AES, RSA, TLS, hashing)
    Encryption,
    /// Input validation / sanitization (XSS, SQLi prevention)
    InputValidation,
    /// Audit logging / forensic trail
    AuditLog,

    // ---- Testing ----
    /// Unit test / test case
    UnitTest,
    /// Integration test / end-to-end test
    IntegrationTest,
    /// Assertion / expectation
    Assertion,
    /// Mock / stub / fake / test double
    MockObject,
    /// Property-based testing / fuzzing
    PropertyTest,
}

impl UniversalConstruct {
    /// Returns the paradigms this construct is most associated with.
    pub fn paradigms(&self) -> Vec<Paradigm> {
        debuglog!("UniversalConstruct::paradigms: {:?}", self);
        match self {
            Self::ClassDefinition | Self::InterfaceDefinition |
            Self::Inheritance | Self::MethodDispatch |
            Self::AccessControl | Self::Constructor | Self::Destructor => {
                vec![Paradigm::ObjectOriented]
            }
            Self::Lambda | Self::HigherOrderFunction |
            Self::PatternMatch | Self::TailCall | Self::Recursion => {
                vec![Paradigm::Functional]
            }
            Self::HdlRegister | Self::HdlWire |
            Self::HdlClockDomain | Self::HdlModuleInstance |
            Self::HdlTestbench => {
                vec![Paradigm::HardwareDescription]
            }
            Self::ThreadSpawn | Self::MutexLock | Self::Channel |
            Self::Actor | Self::Future | Self::AtomicOp => {
                vec![Paradigm::Concurrent]
            }
            Self::PointerReference | Self::ManualMemory |
            Self::OwnershipBorrowing | Self::HeapAllocation => {
                vec![Paradigm::Systems]
            }
            Self::UIComponent | Self::StateManagement |
            Self::Routing | Self::Template => {
                vec![Paradigm::Reactive]
            }
            Self::DatabaseQuery | Self::Styling => {
                vec![Paradigm::Declarative]
            }
            Self::AsyncAwait => {
                vec![Paradigm::Concurrent, Paradigm::Reactive]
            }
            _ => {
                // Most constructs are cross-paradigm
                vec![Paradigm::Procedural]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paradigm_classification_oop() {
        let p = UniversalConstruct::ClassDefinition.paradigms();
        assert!(p.contains(&Paradigm::ObjectOriented));
    }

    #[test]
    fn test_paradigm_classification_functional() {
        let p = UniversalConstruct::Lambda.paradigms();
        assert!(p.contains(&Paradigm::Functional));
    }

    #[test]
    fn test_paradigm_classification_hdl() {
        let p = UniversalConstruct::HdlRegister.paradigms();
        assert!(p.contains(&Paradigm::HardwareDescription));
    }

    #[test]
    fn test_paradigm_classification_concurrent() {
        let p = UniversalConstruct::Channel.paradigms();
        assert!(p.contains(&Paradigm::Concurrent));
    }

    #[test]
    fn test_paradigm_classification_cross_paradigm() {
        // AsyncAwait spans Concurrent + Reactive
        let p = UniversalConstruct::AsyncAwait.paradigms();
        assert!(p.contains(&Paradigm::Concurrent));
        assert!(p.contains(&Paradigm::Reactive));
    }

    #[test]
    fn test_platform_target_variants() {
        // Ensure all platforms are constructible
        let targets = vec![
            PlatformTarget::Linux,
            PlatformTarget::Windows,
            PlatformTarget::MacOS,
            PlatformTarget::IOS,
            PlatformTarget::Android,
            PlatformTarget::Web,
            PlatformTarget::Embedded,
            PlatformTarget::FPGA,
            PlatformTarget::Cloud,
            PlatformTarget::CrossPlatform,
        ];
        assert_eq!(targets.len(), 10);
    }
}
