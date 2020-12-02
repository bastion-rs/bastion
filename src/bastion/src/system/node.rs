#[derive(Debug)]
/// An implementation of the Bastion's node. By default  
/// it's available as a singleton object. The node stores all
/// required information for running various actor implementations,
///  
///
/// Out-of-the-box it also provides:
/// - Adding actor definitions in runtime
/// - API for starting, stopping or terminating actors
/// - Message dispatching  
pub struct Node;

impl Node {
    /// Returns a new instance of the Bastion Node.
    pub(crate) fn new() -> Self {
        Node {}
    }

    // TODO: Add errors handling?
    /// Initializes the Bastion node if it hasn't already been done.
    pub async fn init(&self) {}
}
