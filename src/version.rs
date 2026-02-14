//!
//!  Version checks
//!
//!  This is a best effort for comparing non SemVer compatible
//!  version scheme
//!
use std::borrow::Cow;
use std::char;

use semver::VersionReq;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Version<'a> {
    version: SemVer,
    text: Cow<'a, str>,
}

impl<'a> Version<'a> {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_ref()
    }

    #[inline]
    pub fn exact_match<T: AsRef<str>>(&self, text: T) -> bool {
        self.text == text.as_ref()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum SemVer {
    Full(semver::Version),
    Partial(semver::Version),
    None,
}

impl<T: AsRef<str>> From<T> for SemVer {
    fn from(text: T) -> SemVer {
        SemVer::new(text.as_ref())
    }
}

impl SemVer {
    /// Try to convert into SemVer compatible scheme
    pub fn new(text: &str) -> Self {
        let mut text = text;
        // Strip prefixes
        for pat in ["ver.", "ver", "v.", "v", "Ver", "Ver.", "V", "V."] {
            text = text.trim_start_matches(pat);
        }

        semver::Version::parse(text)
            .ok()
            .map(SemVer::Full)
            .or_else(|| {
                // Attempt to extract the X.Y.Z part of the version string
                let mut count = 0;
                text.split(|c: char| {
                    if c == '.' {
                        count += 1;
                        // Stop a the third separator
                        count >= 3
                    } else {
                        // Stop if hit a non-numeric char
                        !c.is_numeric()
                    }
                })
                .next()
                .and_then(|prefix| {
                    if prefix.ends_with('.') {
                        // Stopped with a non numeric char after a '.'
                        count -= 1
                    }
                    let (_, rest) = text.split_at(prefix.len());
                    let prefix = prefix.trim_end_matches('.');
                    semver::Version::parse(&match count {
                        0 => format!("{prefix}.0.0"),
                        1 => format!("{prefix}.0"),
                        _ => prefix.to_string(),
                    })
                    .ok()
                    .map(|mut v| {
                        // Check if pre is a valid prerelease
                        let pre = semver::Prerelease::new(rest.trim_start_matches('-')).ok();
                        if let Some(pre) = pre {
                            v.pre = pre;
                            SemVer::Full(v)
                        } else {
                            SemVer::Partial(v)
                        }
                    })
                })
            })
            .unwrap_or(SemVer::None)
    }
}

// From trait

impl<'a> From<&'a str> for Version<'a> {
    fn from(text: &'a str) -> Version<'a> {
        Self {
            version: text.into(),
            text: text.into(),
        }
    }
}

impl<'a> From<String> for Version<'a> {
    fn from(text: String) -> Version<'a> {
        Self {
            version: text.as_str().into(),
            text: text.into(),
        }
    }
}

impl<'a> From<&'a String> for Version<'a> {
    fn from(text: &'a String) -> Version<'a> {
        Self {
            version: text.as_str().into(),
            text: text.into(),
        }
    }
}

// Match

pub enum Match<'a> {
    Request(VersionReq),
    Exact(Cow<'a, str>),
}

impl<'a> Match<'a> {
    pub fn parse(&self, text: &str) -> Result<Self, semver::Error> {
        Ok(Self::Request(VersionReq::parse(text)?))
    }

    pub fn new_exact(&self, text: Cow<'a, str>) -> Self {
        Self::Exact(text)
    }

    pub fn matches(&self, ver: &Version<'_>) -> bool {
        match self {
            Self::Request(req) => {
                // If this is a semver request
                // then we assume that the version is also, at least partially,
                // semver compatible, otherwise comparison cannot occurs.
                match &ver.version {
                    SemVer::Full(v) | SemVer::Partial(v) => req.matches(v),
                    _ => false,
                }
            }
            // Perform exact match
            Self::Exact(s) => ver.exact_match(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        fn full(expected: &str) -> SemVer {
            SemVer::Full(semver::Version::parse(expected).unwrap())
        }

        fn partial(expected: &str) -> SemVer {
            SemVer::Partial(semver::Version::parse(expected).unwrap())
        }

        assert_eq!(SemVer::from("1.0.0"), full("1.0.0"));
        assert_eq!(SemVer::from("v2.4.0"), full("2.4.0"));
        assert_eq!(SemVer::from("ver2.4.0"), full("2.4.0"));
        assert_eq!(SemVer::from("v.2.4.0"), full("2.4.0"));
        assert_eq!(SemVer::from("ver.2.4.0"), full("2.4.0"));
        assert_eq!(SemVer::from("2.4.0.1"), partial("2.4.0"));
        assert_eq!(SemVer::from("23.2a"), full("23.2.0-a"));
        assert_eq!(SemVer::from("release"), SemVer::None);
        assert_eq!(SemVer::from("0.6-beta3"), full("0.6.0-beta3"));
        assert_eq!(SemVer::from("0.1.a"), full("0.1.0-a"));
        assert_eq!(SemVer::from("12a"), full("12.0.0-a"));
        assert_eq!(SemVer::from("1.2a.1-beta.2"), full("1.2.0-a.1-beta.2"));
        assert_eq!(SemVer::from("1.2a.1-beta_2"), partial("1.2.0"));
    }

    #[test]
    fn test_version_compare() {
        assert!(Version::from("23.2a") < Version::from("23.2b"));
        assert!(Version::from("alpha") < Version::from("beta"));
        assert!(Version::from("1.2.0-beta_2") < Version::from("1.2.2-beta_1"));
        assert!(Version::from("1.2.0-beta_2") > Version::from("1.2.0-beta_1"));
    }
}
