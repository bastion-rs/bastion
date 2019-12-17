//!
//! A path represents a message sender's semantics and
//! later will be used to route messages to them

use crate::context::{BastionId, NIL_ID};
use std::fmt;
use std::result::Result;

#[derive(Clone)]
/// Represents a Path for a System, Supervisor, Children or Child.
///
/// BastionPath can be used to identify message senders.
/// Later it will be used to route messages to a path.
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
///
///     # Bastion::children(|children| {
///         # children.with_exec(|ctx: BastionContext| {
///             # async move {
/// ctx.tell(&ctx.signature(), "Hello to myself").expect("Couldn't send a message");
/// msg! { ctx.recv().await?,
///     ref msg: &'static str => {
///         let path: &BastionPath = signature!().path();
///         assert_eq!(path.elem(), ctx.signature().path().elem());
///     };
///     // We are only sending a `&'static str` in this
///     // example, so we know that this won't happen...
///     _: _ => ();
/// }
///                 # Bastion::stop();
///                 # Ok(())
///             # }
///         # })
///     # }).unwrap();
///     #
///     # Bastion::start();
///     # Bastion::block_until_stopped();
/// # }
/// ```
pub struct BastionPath {
    // TODO: possibly more effective collection depending on how we'll use it in routing
    parent_chain: Vec<BastionId>,
    this: Option<BastionPathElement>,
}

impl BastionPath {
    // SYSTEM or a sender out of Bastion scope
    pub(crate) fn root() -> BastionPath {
        BastionPath {
            parent_chain: vec![],
            this: None,
        }
    }

    /// iterates over path elements
    pub(crate) fn iter(&self) -> impl Iterator<Item = &BastionId> {
        let parent_iter = self.parent_chain.iter();
        parent_iter.chain(self.this.iter().map(|e| e.id()))
    }

    /// Returns the last element's id.
    /// If it's root or a dead_letters then &NIL_ID is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let children_ref = Bastion::children(|children| children).unwrap();
    /// let msg = "A message containing data.";
    /// children_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         let path = signature!().path();
    ///         assert_eq!(path.id(), &NIL_ID);
    ///     };
    ///     // We are only sending a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn id(&self) -> &BastionId {
        self.this.as_ref().map(|e| e.id()).unwrap_or(&NIL_ID)
    }

    /// Returns a path element. If the path is root then None is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let children_ref = Bastion::children(|children| children).unwrap();
    /// let msg = "A message containing data.";
    /// children_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         let path = signature!().path();
    ///         assert!(path.elem().as_ref().unwrap().is_children());
    ///     };
    ///     // We are only sending a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn elem(&self) -> &Option<BastionPathElement> {
        &self.this
    }

    /// Checks whether `BastionPath` is a dead-letters path.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let children_ref = Bastion::children(|children| children).unwrap();
    /// let msg = "A message containing data.";
    /// children_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         let path = signature!().path();
    ///         assert!(path.is_dead_letters());
    ///     };
    ///     // We are only sending a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn is_dead_letters(&self) -> bool {
        self.parent_chain.len() == 2 && self.this.as_ref().map(|e| e.is_child()).unwrap_or(false)
    }
}

impl fmt::Display for BastionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "/{}",
            self.iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<String>>()
                .join("/")
        )
    }
}

impl fmt::Debug for BastionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.this {
            Some(this @ BastionPathElement::Supervisor(_)) => write!(
                f,
                "/{}",
                self.parent_chain
                    .iter()
                    .map(|id| BastionPathElement::Supervisor(id.clone()))
                    .chain(vec![this.clone()])
                    .map(|el| format!("{:?}", el))
                    .collect::<Vec<String>>()
                    .join("/")
            ),
            // TODO: combine with the pattern above when or-patterns become stable
            Some(this @ BastionPathElement::Children(_)) => write!(
                f,
                "/{}",
                self.parent_chain
                    .iter()
                    .map(|id| BastionPathElement::Supervisor(id.clone()))
                    .chain(vec![this.clone()])
                    .map(|el| format!("{:?}", el))
                    .collect::<Vec<String>>()
                    .join("/")
            ),
            Some(this @ BastionPathElement::Child(_)) => {
                let parent_len = self.parent_chain.len();

                write!(
                    f,
                    "/{}",
                    self.parent_chain
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                            if i == parent_len - 1 {
                                BastionPathElement::Children(id.clone())
                            } else {
                                BastionPathElement::Supervisor(id.clone())
                            }
                        })
                        .chain(vec![this.clone()])
                        .map(|el| format!("{:?}", el))
                        .collect::<Vec<String>>()
                        .join("/")
                )
            }
            None => write!(f, "/"),
        }
    }
}

#[derive(Clone, PartialEq)]
/// Represents BastionPath element
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
///     #
///
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             ctx.tell(&ctx.signature(), "Hello to myself");
///             
///             let msg: SignedMessage = ctx.recv().await?;
///             let elem: &Option<BastionPathElement> = msg.signature().path().elem();
///             assert!(elem.is_some());
///             assert_eq!(elem, ctx.signature().path().elem());
///
///             # Bastion::stop();
///             Ok(())
///         }
///     })
/// }).expect("Couldn't create the children group.");
///     #
///     # Bastion::start();
///     # Bastion::block_until_stopped();
/// # }
/// ```
pub enum BastionPathElement {
    #[doc(hidden)]
    /// Supervisor element
    Supervisor(BastionId),
    #[doc(hidden)]
    /// Children element
    Children(BastionId),
    #[doc(hidden)]
    /// Child element
    Child(BastionId),
}

impl fmt::Debug for BastionPathElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BastionPathElement::Supervisor(id) => write!(f, "supervisor#{}", id),
            BastionPathElement::Children(id) => write!(f, "children#{}", id),
            BastionPathElement::Child(id) => write!(f, "child#{}", id),
        }
    }
}

impl BastionPathElement {
    pub(crate) fn id(&self) -> &BastionId {
        match self {
            BastionPathElement::Supervisor(id) => id,
            BastionPathElement::Children(id) => id,
            BastionPathElement::Child(id) => id,
        }
    }

    pub(crate) fn with_id(self, id: BastionId) -> Self {
        match self {
            BastionPathElement::Supervisor(_) => BastionPathElement::Supervisor(id),
            BastionPathElement::Children(_) => BastionPathElement::Children(id),
            BastionPathElement::Child(_) => BastionPathElement::Child(id),
        }
    }

    #[doc(hidden)]
    /// Checks whether the BastionPath identifies a supervisor.
    pub fn is_supervisor(&self) -> bool {
        match self {
            BastionPathElement::Supervisor(_) => true,
            _ => false,
        }
    }

    #[doc(hidden)]
    /// Checks whether the BastionPath identifies children.
    pub fn is_children(&self) -> bool {
        match self {
            BastionPathElement::Children(_) => true,
            _ => false,
        }
    }

    /// Checks whether the BastionPath identifies a child.
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             ctx.tell(&ctx.signature(), "Hello to myself");
    ///             
    ///             let msg: SignedMessage = ctx.recv().await?;
    ///             let elem: &Option<BastionPathElement> = msg.signature().path().elem();
    ///             assert!(elem.is_some());
    ///             assert_eq!(elem, ctx.signature().path().elem());
    ///
    ///             # Bastion::stop();
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn is_child(&self) -> bool {
        match self {
            BastionPathElement::Child(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct AppendError {
    path: BastionPath,
    element: BastionPathElement,
}

impl fmt::Display for AppendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.element {
            BastionPathElement::Supervisor(..) => match self.path.this {
                None => unreachable!(),
                Some(BastionPathElement::Supervisor(..)) => unreachable!(),
                Some(BastionPathElement::Children(..)) => {
                    write!(f, "Supervisor is not appendable to children")
                }
                Some(BastionPathElement::Child(..)) => {
                    write!(f, "Supervisor is not appendable to a child")
                }
            },
            BastionPathElement::Children(..) => match self.path.this {
                None => write!(f, "Children is not appendable to root"),
                Some(BastionPathElement::Supervisor(..)) => unreachable!(),
                Some(BastionPathElement::Children(..)) => {
                    write!(f, "Children is not appendable to children")
                }
                Some(BastionPathElement::Child(..)) => {
                    write!(f, "Children is not appendable to a child")
                }
            },
            BastionPathElement::Child(..) => match self.path.this {
                None => write!(f, "Child is not appendable to root"),
                Some(BastionPathElement::Supervisor(..)) => {
                    write!(f, "Child is not appendable to a supervisor")
                }
                Some(BastionPathElement::Children(..)) => unreachable!(),
                Some(BastionPathElement::Child(..)) => {
                    write!(f, "Child is not appendable to a child")
                }
            },
        }
    }
}

impl fmt::Debug for AppendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Can't append {:?} to {:?}", self.element, self.path)
    }
}

impl BastionPath {
    pub(crate) fn append(self, el: BastionPathElement) -> Result<BastionPath, AppendError> {
        match el {
            sv @ BastionPathElement::Supervisor(_) => match self.this {
                None => Ok(BastionPath {
                    parent_chain: self.parent_chain,
                    this: Some(sv),
                }),
                Some(BastionPathElement::Supervisor(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(sv),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: sv,
                }),
            },
            children @ BastionPathElement::Children(_) => match self.this {
                Some(BastionPathElement::Supervisor(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(children),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: children,
                }),
            },
            child @ BastionPathElement::Child(_) => match self.this {
                Some(BastionPathElement::Children(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(child),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: child,
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SYSTEM + smth

    #[test]
    fn append_sv_to_system() {
        let sv_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap();
        assert_eq!(path.iter().collect::<Vec<&BastionId>>(), vec![&sv_id]);
    }

    #[test]
    fn append_children_to_system() {
        let sv_id = BastionId::new();
        let res = BastionPath::root().append(BastionPathElement::Children(sv_id));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Children is not appendable to root"
        );
    }

    #[test]
    fn append_child_to_system() {
        let sv_id = BastionId::new();
        let res = BastionPath::root().append(BastionPathElement::Child(sv_id));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Child is not appendable to root"
        );
    }

    // Supervisor + smth

    #[test]
    fn append_sv_to_sv() {
        let sv1_id = BastionId::new();
        let sv2_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv1_id.clone()))
            .unwrap()
            .append(BastionPathElement::Supervisor(sv2_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv1_id, &sv2_id]
        );
    }

    #[test]
    fn append_children_to_sv() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap()
            .append(BastionPathElement::Children(children_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv_id, &children_id]
        );
    }

    #[test]
    fn append_child_to_sv() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Child(children_id));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Child is not appendable to a supervisor"
        );
    }

    // children + smth

    #[test]
    fn append_sv_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Supervisor(BastionId::new()));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Supervisor is not appendable to children"
        );
    }

    #[test]
    fn append_children_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Children(BastionId::new()));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Children is not appendable to children"
        );
    }

    #[test]
    fn append_child_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap()
            .append(BastionPathElement::Children(children_id.clone()))
            .unwrap()
            .append(BastionPathElement::Child(child_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv_id, &children_id, &child_id]
        );
    }

    // child + smth

    #[test]
    fn append_sv_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Supervisor(BastionId::new()));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Supervisor is not appendable to a child"
        );
    }

    #[test]
    fn append_children_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Children(BastionId::new()));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Children is not appendable to a child"
        );
    }

    #[test]
    fn append_child_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Child(BastionId::new()));
        assert_eq!(
            res.unwrap_err().to_string(),
            "Child is not appendable to a child"
        );
    }
}
