use std::path::Path;

pub struct UriPath<'a> {
    prefix: &'a str,
    components: Vec<String>,
}

impl ToString for UriPath<'_> {
    fn to_string(&self) -> String {
        format!("{}{}", self.prefix, self.components.join("/"))
    }
}

impl<'a> UriPath<'a> {
    fn new(prefix: &'a str, path: &'_ str) -> Self {
        let components = path
            .split('/')
            .filter(|it| *it != "." && *it != "")
            .map(|it| it.to_string())
            .collect();
        UriPath { prefix, components }
    }
    pub fn join(&'a self, component: &'_ str) -> Self {
        let mut components = self.components.to_vec();
        components.push(component.to_string());
        UriPath {
            prefix: self.prefix,
            components,
        }
    }
    pub fn parent(&'a self) -> Option<Self> {
        let mut components = self.components.to_vec();
        if components.len() > 1 {
            let _ = components.pop();
            Some(UriPath {
                prefix: self.prefix,
                components,
            })
        } else {
            None
        }
    }
    pub fn from(prefix: &'a str, root: &'_ str, path: &'_ Path) -> Option<Self> {
        path.to_str().and_then(|path| {
            if !path.starts_with(root) {
                return None;
            }
            Some(Self::new(prefix, &path[root.len()..]))
        })
    }
}
