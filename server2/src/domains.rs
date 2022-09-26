use std::string::ToString;

pub const LOCALHOST: &'static str = "localhost";
pub const LOCALHOST_IPV4: &'static str = "127.0.0.1";
pub const LOCALHOST_IPV6: &'static str = "::1";
pub const APEX: &'static str = "packurl.net";
pub const WWW: &'static str = "www.packurl.net";
pub const CDN: &'static str = "cdn.packurl.net";
pub const SELF_SIGNED_DOMAINS: DomainList = DomainList {
    domains: &[CDN, LOCALHOST, LOCALHOST, LOCALHOST_IPV6],
};
pub const ACME_DOMAINS: DomainList = DomainList {
    domains: &[APEX, WWW],
};

pub struct DomainList {
    domains: &'static [&'static str],
}

impl Into<Vec<String>> for DomainList {
    fn into(self) -> Vec<String> {
        self.domains.iter().map(|it| it.to_string()).collect()
    }
}

impl DomainList {
    pub fn find(&self, needle: &str) -> Option<()> {
        self.domains.iter().find(|&&it| it == needle).map(|_| ())
    }
}
