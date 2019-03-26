use super::{CliError, LalResult, Manifest};
use std::fmt;
use std::path::PathBuf;

/// Set the channel. The channel will be tidied before use, to ensure it
/// is valid. A channel are rejected iff a component of the channel's path
/// exactly matches "testing", and allow_testing is false.
pub fn set(manifest: &mut Manifest, path: &str, allow_testing: bool) -> LalResult<()> {
    let channel = Channel::new(path);
    channel.verify()?;
    if !allow_testing && channel.is_testing() {
        return Err(CliError::InvalidTestingChannel(channel.to_string()));
    }
    debug!("Storing channel: {}", channel);
    manifest.channel = Some(channel.to_string());
    manifest.write()
}

/// Ensure a channel is correctly formatted. e.g. a//b/c/ goes to /a/b/c
pub fn tidy_channel(path: &str) -> String { Channel::new(path).to_string() }

fn path_to_vec(channel: &str) -> Vec<String> {
    component_iter(channel).map(ToOwned::to_owned).collect()
}

const DEFAULT_CHANNEL: &str = "/";

/// Structure representing a channel's path.
#[derive(Debug, Eq, Clone)]
pub struct Channel {
    components: Vec<String>,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "/{}", self.components.join(&"/"))
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool { self.components == other.components }
}

impl Default for Channel {
    fn default() -> Self { Self::new(DEFAULT_CHANNEL) }
}

impl Channel {
    /// Identifier for a testing channel.
    pub const TESTING: &'static str = "testing";

    /// Creates a new Channel object with a given path.
    pub fn new(path: &str) -> Self {
        Self {
            components: path_to_vec(path),
        }
    }

    /// Verifies that the channel is valid. A channel is invalid
    /// iff any of the following are true:
    /// - A non-leaf component in the channel path is "testing"
    /// - The channel contains a NUL character
    pub fn verify(&self) -> LalResult<()> {
        if self.components.len() > 1 {
            for c in &self.components[0..self.components.len() - 1] {
                if *c == Self::TESTING {
                    return Err(CliError::InvalidTestingChannel(self.to_string()));
                }
            }
        }

        for c in &self.components {
            for b in c.bytes() {
                if b == b'\0' {
                    return Err(CliError::InvalidChannelCharacter(self.to_string()));
                }
            }

            if c.is_empty() {
                unreachable!("Empty channel component - this should not be able to happen")
            }
        }
        Ok(())
    }

    /// Creates a new Channel with a given path if provided, else uses a default path.
    pub fn from_option<S>(path: &Option<S>) -> Self
    where
        S: ToString,
    {
        match path {
            Some(path) => Self::new(&path.to_string()),
            None => Self::default(),
        }
    }

    /// Verifies whether one channel contains the other, and therefore
    /// is a valid parent in the channel hierarchy.
    pub fn contains(&self, other: &Self) -> LalResult<()> {
        // Zip truncates to the shortest length. The rest of the verification
        // will therefore be invalid if the other is longer than self.
        if other.components.len() > self.components.len() {
            return Err(CliError::ChannelMismatch(
                other.to_string(),
                self.to_string(),
            ));
        }

        let mut iter = self
            .components
            .iter()
            .zip(other.components.iter())
            .peekable();
        while iter.peek().is_some() {
            let (a, b) = iter.next().unwrap();
            let is_last = iter.peek().is_none();

            // Testing branches support a special case of contains,
            // where "/a/testing" is considered a parent of "/a/b/testing".
            if self.is_testing() && *b == Self::TESTING && is_last {
                break;
            }

            // If the path is different then b cannot contain a.
            if *a != *b {
                return Err(CliError::ChannelMismatch(
                    self.to_string(),
                    other.to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Verifies whether a channel is designated as a testing channel.
    pub fn is_testing(&self) -> bool {
        if let Some(ch) = self.components.last() {
            *ch == Self::TESTING
        } else {
            false
        }
    }

    fn convert(&self, prefix: &str, join: &str, suffix: &str) -> String {
        if self.components.is_empty() {
            String::new()
        } else {
            format!("{}{}{}", prefix, self.components.join(join), suffix)
        }
    }

    /// Provides a formatted string suitable for insertion into a web address.
    pub fn http_string(&self) -> String { self.convert("ch/", "/ch/", "/") }

    /// Provides a formatted string suitable for insertion into a file path.
    pub fn fs_string(&self) -> String { self.convert("/channels/", "/channels/", "") }

    /// Provides a formatted string suitable for insertion into a version string (i.e. repository=/chan/version)
    pub fn version_string(&self) -> String { self.convert("/", "/", "/") }

    /// Converts the channel to an owned path object.
    pub fn to_path(&self) -> PathBuf { PathBuf::from(&self.convert("channels/", "/channels/", "")) }
}

fn component_iter(channel: &str) -> impl Iterator<Item = &str> {
    channel.split('/').filter(|s| !s.is_empty())
}

/// Returns the current channel path. If no channel is found then the default channel is returned.
pub fn get(manifest: &Manifest) -> &str {
    if let Some(ref channel) = manifest.channel {
        &channel
    } else {
        DEFAULT_CHANNEL
    }
}

/// Retrieves coordinates from a coordinate string.
pub fn parse_coords(coords: &str) -> (Option<u32>, Option<Channel>) {
    let split_coords = coords.split('/').collect::<Vec<_>>();
    let version = split_coords.last().unwrap().parse::<u32>().ok();

    // The parsing works as follows for coords /a/b:
    // - If b is a number:
    // * Channel: /a
    // * Version: b
    // - Otherwise:
    // * Channel: /a/b
    // * Version: None
    let channel = if split_coords.len() == 1 {
        // There is no slash
        None
    } else {
        let path = if version.is_some() {
            split_coords
                .iter()
                .take(split_coords.len() - 1)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            split_coords
        }
        .join(&"/");
        Some(Channel::new(&path))
    };
    (version, channel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse() {
        fn assert_parse(input: &str, version: Option<u32>, channel: Option<&str>) {
            let coords = parse_coords(input);
            assert_eq!(coords.0, version);
            if let Some(channel) = channel {
                assert!(coords.1.is_some());
                assert_eq!(coords.1.unwrap(), Channel::new(&channel));
            } else {
                assert!(coords.1.is_none());
            }
        }

        assert_parse("", None, None);
        assert_parse("a", None, None);
        assert_parse("1", Some(1), None);
        assert_parse("/a", None, Some("/a"));
        assert_parse("/a/", None, Some("/a"));
        assert_parse("/1", Some(1), Some("/"));
        assert_parse("/1/", None, Some("/1"));
        assert_parse("/a/1", Some(1), Some("/a"));
        assert_parse("/a/1/", None, Some("/a/1"));
        assert_parse("/1/2", Some(2), Some("/1"));
        assert_parse("/1/2/", None, Some("/1/2"));
    }

    #[test]
    fn test_tidy() {
        fn assert_tidy(input: &str, output: &str) {
            assert_eq!(Channel::new(input).to_string(), output);
            assert_eq!(tidy_channel(input), output);
        }

        assert_tidy("", DEFAULT_CHANNEL);
        assert_tidy("/", "/");
        assert_tidy("//", "/");
        assert_tidy("///", "/");
        assert_tidy("a", "/a");
        assert_tidy("µ", "/µ");
        assert_tidy("µ/Ø///", "/µ/Ø");
        assert_tidy("aa", "/aa");
        assert_tidy("////a///", "/a");
        assert_tidy("a/b", "/a/b");
        assert_tidy("a////b/c/d////", "/a/b/c/d");
    }

    #[test]
    fn test_http() {
        fn assert_http(input: Channel, output: &str) {
            assert_eq!(input.http_string(), output);
        }

        let input = Channel::default();
        let output = "";
        assert_http(input, output);

        let input = Channel::new("a");
        let output = "ch/a/";
        assert_http(input, output);

        let input = Channel::new("/a/b/c");
        let output = "ch/a/ch/b/ch/c/";
        assert_http(input, output);
    }

    #[test]
    fn test_fs() {
        fn assert_fs(input: Channel, output: &str) {
            assert_eq!(input.fs_string(), output);
        }

        let input = Channel::default();
        let output = "";
        assert_fs(input, output);

        let input = Channel::new("a");
        let output = "/channels/a";
        assert_fs(input, output);

        let input = Channel::new("/a/b/c");
        let output = "/channels/a/channels/b/channels/c";
        assert_fs(input, output);
    }

    #[test]
    fn test_path() {
        fn assert_path(input: Channel, output: &str) {
            assert_eq!(input.to_path(), Path::new(output))
        }

        let input = Channel::default();
        let output = "";
        assert_path(input, output);

        let input = Channel::new("a");
        let output = "channels/a";
        assert_path(input, output);

        let input = Channel::new("/a/b/c");
        let output = "channels/a/channels/b/channels/c";
        assert_path(input, output);
    }

    #[test]
    fn test_contains() {
        fn assert_contains(child: Channel, parent: Channel) {
            assert!(child.contains(&parent).is_ok())
        }

        fn assert_separate(child: Channel, parent: Channel) {
            assert!(child.contains(&parent).is_err())
        }

        let child = Channel::default();
        let parent = Channel::default();
        assert_contains(child, parent);

        let child = Channel::new("/a");
        let parent = Channel::new("/a");
        assert_contains(child, parent);

        let child = Channel::new("/a");
        let parent = Channel::new("/b");
        assert_separate(child, parent);

        let child = Channel::new("/a/b");
        let parent = Channel::new("/a");
        assert_contains(child, parent);

        let child = Channel::new("/a/b");
        let parent = Channel::new("/b");
        assert_separate(child, parent);

        let child = Channel::new("/a/testing");
        let parent = Channel::new("/a");
        assert_contains(child, parent);

        let child = Channel::new("/a/testing");
        let parent = Channel::new("/");
        assert_contains(child, parent);

        let child = Channel::new("/a/testing");
        let parent = Channel::new("/b");
        assert_separate(child, parent);

        let child = Channel::new("/a/testing");
        let parent = Channel::new("/b/testing");
        assert_separate(child, parent);

        let child = Channel::new("/a/testing");
        let parent = Channel::new("/a/testing");
        assert_contains(child, parent);

        let child = Channel::new("/a/b/testing");
        let parent = Channel::new("/a/testing");
        assert_contains(child, parent);

        let child = Channel::new("/a/b/testing");
        let parent = Channel::new("/a/c/testing");
        assert_separate(child, parent);

        let child = Channel::new("/a/b/testing");
        let parent = Channel::new("/b/testing");
        assert_separate(child, parent);
    }

    #[test]
    fn test_testing() {
        let ch = Channel::default();
        assert!(!ch.is_testing());

        let ch = Channel::new("/a/b/c");
        assert!(!ch.is_testing());

        let ch = Channel::new("/testing");
        assert!(ch.is_testing());

        let ch = Channel::new("/a/testing");
        assert!(ch.is_testing());
    }

    #[test]
    fn test_verify() {
        let ch = Channel::default();
        assert!(ch.verify().is_ok());

        let ch = Channel::new("/a/b");
        assert!(ch.verify().is_ok());

        let ch = Channel::new("\n");
        assert!(ch.verify().is_ok());

        let ch = Channel::new("\0");
        match ch.verify() {
            Err(CliError::InvalidChannelCharacter(_)) => (),
            _ => assert!(false),
        }

        let ch = Channel::new("/testing");
        assert!(ch.verify().is_ok());

        let ch = Channel::new("/a/testing");
        assert!(ch.verify().is_ok());

        let ch = Channel::new("/testing/a");
        match ch.verify() {
            Err(CliError::InvalidTestingChannel(_)) => (),
            _ => assert!(false),
        }
    }
}
