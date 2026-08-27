use std::fmt;

/// An expanded XML name: namespace URI (`None` for an unqualified name) plus a
/// local name.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name {
    pub ns: Option<String>,
    pub local: String,
}

impl Name {
    pub fn new(ns: Option<impl Into<String>>, local: impl Into<String>) -> Self {
        Name {
            ns: ns.map(Into::into),
            local: local.into(),
        }
    }

    pub fn qualified(ns: impl Into<String>, local: impl Into<String>) -> Self {
        Name {
            ns: Some(ns.into()),
            local: local.into(),
        }
    }

    pub fn unqualified(local: impl Into<String>) -> Self {
        Name {
            ns: None,
            local: local.into(),
        }
    }

    pub fn is(&self, ns: &str, local: &str) -> bool {
        self.local == local && self.ns.as_deref() == Some(ns)
    }

    pub fn in_ns(&self, ns: &str) -> bool {
        self.ns.as_deref() == Some(ns)
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ns {
            Some(ns) => write!(f, "{{{ns}}}{}", self.local),
            None => write!(f, "{}", self.local),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub name: Name,
    pub value: String,
}

impl Attribute {
    pub fn new(name: Name, value: impl Into<String>) -> Self {
        Attribute {
            name,
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
    CData(String),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: Name,
    pub attrs: Vec<Attribute>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(name: Name) -> Self {
        Element {
            name,
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    /// An element holding a single text node.
    pub fn with_text(name: Name, text: impl Into<String>) -> Self {
        Element {
            name,
            attrs: Vec::new(),
            children: vec![Node::Text(text.into())],
        }
    }

    pub fn is(&self, ns: &str, local: &str) -> bool {
        self.name.is(ns, local)
    }

    /// True when this element has at least one element child. Used to decide
    /// whether the writer may re-indent its content.
    pub fn has_element_children(&self) -> bool {
        self.children.iter().any(|c| matches!(c, Node::Element(_)))
    }

    pub fn attr(&self, ns: Option<&str>, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name.local == local && a.name.ns.as_deref() == ns)
            .map(|a| a.value.as_str())
    }

    pub fn set_attr(&mut self, name: Name, value: impl Into<String>) {
        let value = value.into();
        match self.attrs.iter_mut().find(|a| a.name == name) {
            Some(a) => a.value = value,
            None => self.attrs.push(Attribute { name, value }),
        }
    }

    pub fn take_attr(&mut self, ns: Option<&str>, local: &str) -> Option<String> {
        let i = self
            .attrs
            .iter()
            .position(|a| a.name.local == local && a.name.ns.as_deref() == ns)?;
        Some(self.attrs.remove(i).value)
    }

    /// Concatenated text of this element and its descendants.
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        for child in &self.children {
            match child {
                Node::Text(t) | Node::CData(t) => out.push_str(t),
                Node::Element(e) => e.collect_text(out),
                Node::Comment(_) => {}
            }
        }
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|c| match c {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }

    pub fn elements_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.children.iter_mut().filter_map(|c| match c {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }

    pub fn child(&self, ns: &str, local: &str) -> Option<&Element> {
        self.elements().find(|e| e.is(ns, local))
    }

    pub fn push(&mut self, el: Element) {
        self.children.push(Node::Element(el));
    }
}
