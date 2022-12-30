/// A node in a binary tree where every node has 0 or 2 children.
pub enum FullBTreeNode<T> {
    Leaf(T),
    Inner(Box<FullBTreeInnerNode<T>>),
}

// todo: this representation is kinda silly because the tree is always a list
// basically
pub struct FullBTreeInnerNode<T> {
    pub left: FullBTreeNode<T>,
    pub right: FullBTreeNode<T>,
}
