// ============================================================
// Abstract Syntax Tree — Tier 1 Forensic Representation
// Section 1.III: "Generates mathematically perfect ASTs."
//
// The AST is the single source of logical truth. Tier 2
// decorative expansion MUST NOT alter this structure.
//
// Each AstNode carries an optional hypervector fingerprint
// for bidirectional mapping between symbolic and HDC spaces.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdlm::error::HdlmError;

/// Unique identifier for an AST node within a tree.
pub type NodeId = usize;

/// The semantic kind of an AST node.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // -- Program structure --
    /// Root of the entire program/document.
    Root,
    /// A named block (function, method, section).
    Block { name: String },

    // -- Expressions --
    /// A literal value (integer, float, string, bool).
    Literal { value: String },
    /// A named variable or symbol reference.
    Identifier { name: String },
    /// A binary operation.
    BinaryOp { operator: String },
    /// A unary operation.
    UnaryOp { operator: String },
    /// A function/method call.
    Call { function: String },

    // -- Statements --
    /// Variable binding / assignment.
    Assignment,
    /// Return statement.
    Return,
    /// Conditional branch.
    Conditional,
    /// Loop construct.
    Loop,

    // -- Natural language (for HDLM prose generation) --
    /// A sentence or clause in natural language output.
    Sentence,
    /// A discrete phrase/term.
    Phrase { text: String },
}

/// A single node in the AST.
#[derive(Debug, Clone)]
pub struct AstNode {
    /// Unique ID within this tree.
    pub id: NodeId,
    /// Semantic kind.
    pub kind: NodeKind,
    /// Child node IDs (ordered).
    pub children: Vec<NodeId>,
    /// Optional HDC fingerprint for vector-space mapping.
    pub hv_fingerprint: Option<BipolarVector>,
}

impl AstNode {
    /// Create a new node without children or fingerprint.
    pub fn new(id: NodeId, kind: NodeKind) -> Self {
        debuglog!("AstNode::new: id={}, kind={:?}", id, kind);
        Self {
            id,
            kind,
            children: Vec::new(),
            hv_fingerprint: None,
        }
    }

    /// Create a new node with a hypervector fingerprint.
    pub fn with_fingerprint(id: NodeId, kind: NodeKind, hv: BipolarVector) -> Self {
        debuglog!("AstNode::with_fingerprint: id={}, kind={:?}", id, kind);
        Self {
            id,
            kind,
            children: Vec::new(),
            hv_fingerprint: Some(hv),
        }
    }

    /// Whether this node has children.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// The full AST. Arena-allocated: nodes stored in a Vec, referenced by NodeId.
#[derive(Debug, Clone)]
pub struct Ast {
    nodes: Vec<AstNode>,
    root_id: Option<NodeId>,
}

impl Ast {
    /// Create an empty AST.
    pub fn new() -> Self {
        debuglog!("Ast::new: empty tree");
        Self {
            nodes: Vec::new(),
            root_id: None,
        }
    }

    /// Number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the root node ID, if set.
    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    /// Add a node to the arena. Returns its NodeId.
    /// The first node added with kind Root is auto-set as root.
    pub fn add_node(&mut self, kind: NodeKind) -> NodeId {
        let id = self.nodes.len();
        let is_root = matches!(kind, NodeKind::Root) && self.root_id.is_none();
        let node = AstNode::new(id, kind);
        self.nodes.push(node);

        if is_root {
            self.root_id = Some(id);
            debuglog!("Ast::add_node: root set to id={}", id);
        } else {
            debuglog!("Ast::add_node: id={}", id);
        }

        id
    }

    /// Add a node with a hypervector fingerprint.
    pub fn add_node_with_hv(&mut self, kind: NodeKind, hv: BipolarVector) -> NodeId {
        let id = self.nodes.len();
        let is_root = matches!(kind, NodeKind::Root) && self.root_id.is_none();
        let node = AstNode::with_fingerprint(id, kind, hv);
        self.nodes.push(node);

        if is_root {
            self.root_id = Some(id);
        }
        debuglog!("Ast::add_node_with_hv: id={}", id);
        id
    }

    /// Attach a child to a parent node.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), HdlmError> {
        if parent >= self.nodes.len() || child >= self.nodes.len() {
            debuglog!("Ast::add_child: invalid ids parent={}, child={}", parent, child);
            return Err(HdlmError::MalformedAst {
                reason: format!(
                    "Invalid node IDs: parent={}, child={}, total={}",
                    parent,
                    child,
                    self.nodes.len()
                ),
            });
        }
        if parent == child {
            return Err(HdlmError::MalformedAst {
                reason: "Cannot make a node its own child".to_string(),
            });
        }
        self.nodes[parent].children.push(child);
        debuglog!("Ast::add_child: parent={} -> child={}", parent, child);
        Ok(())
    }

    /// Read-only access to a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&AstNode> {
        self.nodes.get(id)
    }

    /// Depth-first traversal, returning node IDs in visit order.
    pub fn dfs(&self) -> Result<Vec<NodeId>, HdlmError> {
        let root = self.root_id.ok_or(HdlmError::EmptyAst)?;
        let mut visited = Vec::with_capacity(self.nodes.len());
        let mut stack = vec![root];

        while let Some(current) = stack.pop() {
            visited.push(current);
            if let Some(node) = self.nodes.get(current) {
                // Push children in reverse so leftmost is visited first
                for &child in node.children.iter().rev() {
                    stack.push(child);
                }
            }
        }

        debuglog!("Ast::dfs: visited {} nodes", visited.len());
        Ok(visited)
    }

    /// Breadth-first traversal, returning node IDs in level order.
    pub fn bfs(&self) -> Result<Vec<NodeId>, HdlmError> {
        let root = self.root_id.ok_or(HdlmError::EmptyAst)?;
        let mut visited = Vec::with_capacity(self.nodes.len());
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root);

        while let Some(current) = queue.pop_front() {
            visited.push(current);
            if let Some(node) = self.nodes.get(current) {
                for &child in &node.children {
                    queue.push_back(child);
                }
            }
        }

        debuglog!("Ast::bfs: visited {} nodes", visited.len());
        Ok(visited)
    }

    /// Count the total number of leaf nodes.
    pub fn leaf_count(&self) -> usize {
        let count = self.nodes.iter().filter(|n| n.is_leaf()).count();
        debuglog!("Ast::leaf_count: {}", count);
        count
    }
}

// ============================================================
// AST Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_ast() {
        let ast = Ast::new();
        assert!(ast.is_empty());
        assert_eq!(ast.node_count(), 0);
        assert_eq!(ast.root_id(), None);
    }

    #[test]
    fn test_add_root_node() {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        assert_eq!(root, 0);
        assert_eq!(ast.root_id(), Some(0));
        assert_eq!(ast.node_count(), 1);
    }

    #[test]
    fn test_add_child() -> Result<(), HdlmError> {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let child = ast.add_node(NodeKind::Block { name: "main".to_string() });
        ast.add_child(root, child)?;

        let root_node = ast.get_node(root);
        assert!(root_node.is_some());
        assert_eq!(root_node.map(|n| n.children.len()), Some(1));
        Ok(())
    }

    #[test]
    fn test_add_child_self_reference_fails() {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let result = ast.add_child(root, root);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_child_invalid_ids() {
        let mut ast = Ast::new();
        let _ = ast.add_node(NodeKind::Root);
        let result = ast.add_child(0, 99);
        assert!(result.is_err());
    }

    #[test]
    fn test_dfs_traversal() -> Result<(), HdlmError> {
        // Build: Root -> [Block("a"), Block("b")]
        //        Block("a") -> [Literal("1")]
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let a = ast.add_node(NodeKind::Block { name: "a".to_string() });
        let b = ast.add_node(NodeKind::Block { name: "b".to_string() });
        let lit = ast.add_node(NodeKind::Literal { value: "1".to_string() });

        ast.add_child(root, a)?;
        ast.add_child(root, b)?;
        ast.add_child(a, lit)?;

        let order = ast.dfs()?;
        // DFS: root(0), a(1), lit(3), b(2)
        assert_eq!(order, vec![0, 1, 3, 2]);
        Ok(())
    }

    #[test]
    fn test_bfs_traversal() -> Result<(), HdlmError> {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let a = ast.add_node(NodeKind::Block { name: "a".to_string() });
        let b = ast.add_node(NodeKind::Block { name: "b".to_string() });
        let lit = ast.add_node(NodeKind::Literal { value: "1".to_string() });

        ast.add_child(root, a)?;
        ast.add_child(root, b)?;
        ast.add_child(a, lit)?;

        let order = ast.bfs()?;
        // BFS: root(0), a(1), b(2), lit(3)
        assert_eq!(order, vec![0, 1, 2, 3]);
        Ok(())
    }

    #[test]
    fn test_dfs_empty_tree_fails() {
        let ast = Ast::new();
        assert_eq!(ast.dfs(), Err(HdlmError::EmptyAst));
    }

    #[test]
    fn test_leaf_count() -> Result<(), HdlmError> {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let a = ast.add_node(NodeKind::Block { name: "a".to_string() });
        let b = ast.add_node(NodeKind::Literal { value: "x".to_string() });
        let c = ast.add_node(NodeKind::Literal { value: "y".to_string() });

        ast.add_child(root, a)?;
        ast.add_child(root, b)?;
        ast.add_child(a, c)?;

        // Leaves: b(2) and c(3). Root and a have children.
        assert_eq!(ast.leaf_count(), 2);
        Ok(())
    }

    #[test]
    fn test_node_with_fingerprint() -> Result<(), Box<dyn std::error::Error>> {
        let mut ast = Ast::new();
        let hv = BipolarVector::new_random()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let id = ast.add_node_with_hv(NodeKind::Identifier { name: "x".to_string() }, hv);
        let node = ast.get_node(id);
        assert!(node.is_some());
        assert!(node.map(|n| n.hv_fingerprint.is_some()).unwrap_or(false));
        Ok(())
    }

    #[test]
    fn test_is_leaf() {
        let node_leaf = AstNode::new(0, NodeKind::Literal { value: "42".to_string() });
        assert!(node_leaf.is_leaf());

        let mut node_parent = AstNode::new(1, NodeKind::Root);
        node_parent.children.push(0);
        assert!(!node_parent.is_leaf());
    }

    // ============================================================
    // Stress / invariant tests for Ast
    // ============================================================

    /// INVARIANT: add_node assigns sequential IDs starting at 0.
    #[test]
    fn invariant_add_node_sequential_ids() {
        let mut ast = Ast::new();
        for i in 0..10 {
            let id = ast.add_node(NodeKind::Literal { value: format!("{}", i) });
            assert_eq!(id, i, "sequential IDs not preserved: got {} at step {}", id, i);
        }
    }

    /// INVARIANT: add_child rejects self-reference and out-of-bounds IDs.
    #[test]
    fn invariant_add_child_rejects_invalid() {
        let mut ast = Ast::new();
        let a = ast.add_node(NodeKind::Root);
        assert!(ast.add_child(a, a).is_err(), "self-reference should be rejected");
        assert!(ast.add_child(999, 0).is_err(), "out-of-bounds parent rejected");
        assert!(ast.add_child(0, 999).is_err(), "out-of-bounds child rejected");
    }

    /// INVARIANT: dfs and bfs visit exactly the reachable nodes.
    #[test]
    fn invariant_dfs_bfs_same_reachable_set() -> Result<(), Box<dyn std::error::Error>> {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let a = ast.add_node(NodeKind::Phrase { text: "a".into() });
        let b = ast.add_node(NodeKind::Phrase { text: "b".into() });
        ast.add_child(root, a)?;
        ast.add_child(root, b)?;

        let dfs: std::collections::HashSet<_> = ast.dfs()?.into_iter().collect();
        let bfs: std::collections::HashSet<_> = ast.bfs()?.into_iter().collect();
        assert_eq!(dfs, bfs, "dfs and bfs should visit same node set");
        Ok(())
    }

    /// INVARIANT: empty AST dfs/bfs returns EmptyAst error.
    #[test]
    fn invariant_empty_ast_traversals_fail() {
        let ast = Ast::new();
        assert!(ast.dfs().is_err(), "dfs on empty ast should error");
        assert!(ast.bfs().is_err(), "bfs on empty ast should error");
    }

    /// INVARIANT: leaf_count + non_leaf_count == total node count.
    #[test]
    fn invariant_leaf_count_plus_non_leaf_equals_total() -> Result<(), Box<dyn std::error::Error>> {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let c1 = ast.add_node(NodeKind::Phrase { text: "c1".into() });
        let c2 = ast.add_node(NodeKind::Phrase { text: "c2".into() });
        ast.add_child(root, c1)?;
        ast.add_child(root, c2)?;
        let total = ast.node_count();
        let leaves = ast.leaf_count();
        assert!(leaves <= total);
        // c1, c2 are leaves; root is not -> leaves=2, non_leaves=1
        assert_eq!(leaves, 2);
        Ok(())
    }

    /// INVARIANT: add_node_with_hv stores fingerprint and returns sequential id.
    #[test]
    fn invariant_add_with_hv_stores_fingerprint() -> Result<(), Box<dyn std::error::Error>> {
        let mut ast = Ast::new();
        let hv = BipolarVector::new_random()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let id = ast.add_node_with_hv(NodeKind::Literal { value: "x".into() }, hv);
        assert_eq!(id, 0);
        let node = ast.get_node(id).unwrap();
        assert!(node.hv_fingerprint.is_some());
        Ok(())
    }
}
