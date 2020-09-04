use crate::routing::ActorPath;
use lazy_static::lazy_static;
use regex::{escape, CaptureMatches, Captures, Regex};

lazy_static! {
    static ref WILDCARD_REGEX: Regex = Regex::new(r"(\*/)").unwrap();
}

/// An enum for handling targeting messages to the certain
/// actor, a group of actors, a namespace or a scope.
#[derive(Debug, Clone)]
pub enum Target<'a> {
    /// The message must be delivered to the actor(s) by
    /// the matched path.
    Path(ActorPath),
    /// The message must be delivered to actor(s), organized
    /// under the named group.
    Group(&'a str),
}

/// A wrapper for the Target enum type that provides an additional
/// functionality in matching actor paths with the desired pattern.
pub(crate) struct EnvelopeTarget<'a> {
    target: Target<'a>,
    regex: Option<Regex>,
}

impl<'a> EnvelopeTarget<'a> {
    /// Compares the given path with the declared path
    pub(crate) fn is_match(&self, actor_path: &ActorPath) -> bool {
        match &self.target {
            // Just do a regular matching by path, or via the regular
            // expression if the path contains wildcard symbols.
            Target::Path(path) => match &self.regex {
                Some(regex) => {
                    let stringified_path = actor_path.to_string();
                    regex.is_match(&stringified_path)
                }
                None => actor_path == path,
            },
            // False, because the group name is not a part of the actor's path.
            Target::Group(_) => false,
        }
    }

    /// Returns a group name, extracted from the target field.
    pub(crate) fn get_group_name(&self) -> Option<&'a str> {
        match self.target {
            Target::Group(group_name) => Some(group_name),
            _ => None,
        }
    }
}

impl<'a> From<Target<'a>> for EnvelopeTarget<'a> {
    fn from(target: Target<'a>) -> Self {
        let regex = match &target {
            // For a path regex is optional. But it needs to be generated, if
            // the user specified wildcards, such as asterisks symbols.
            Target::Path(path) => {
                let stringified_path = path.to_string();

                match stringified_path.contains("/*") {
                    // Exists at least one wildcard that needs to be replaced
                    // onto the regular expression.
                    true => {
                        let mut raw_regex = match stringified_path.ends_with("/*") {
                            // Necessary to replace the ending, so that any path with
                            // any nested level can be considered as a correct one.
                            true => {
                                let mut fixed_path = stringified_path;
                                let index = fixed_path.rfind("/*").unwrap();
                                fixed_path.replace_range(index.., r"/[[\w\d\-_]/?]+");
                                fixed_path
                            }
                            // Common case: the wildcard closed by the "/" character
                            false => stringified_path,
                        };

                        let raw_regex = format!(
                            "{}$",
                            WILDCARD_REGEX.replace_all(&raw_regex, r"[\w\d\-_.]+/")
                        );
                        let compiled_regex = Regex::new(&raw_regex).unwrap();
                        Some(compiled_regex)
                    }
                    // Wildcard wasn't specified in the path: no needed to
                    // generate a regular expression here.
                    false => None,
                }
            }
            // For a group regex doesn't required. Always do
            // a direct string comparison in dispatchers.
            Target::Group(_) => None,
        };

        EnvelopeTarget { target, regex }
    }
}

#[cfg(test)]
mod message_target_tests {
    use crate::routing::path::ActorPath;
    use crate::routing::target::{EnvelopeTarget, Target};

    #[test]
    fn test_get_group_name_returns_str_reference() {
        let target = Target::Group("test");
        let envelope_target = EnvelopeTarget::from(target);

        let result = envelope_target.get_group_name();
        assert_eq!(result.is_some(), true);
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_get_group_name_returns_none_for_target_path_type() {
        let path = ActorPath::default().name("test");
        let target = Target::Path(path.clone());
        let envelope_target = EnvelopeTarget::from(target);

        let result = envelope_target.get_group_name();
        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn test_match_by_path_with_without_a_wildcard_returns_true() {
        let path = ActorPath::default().name("test");
        let target = Target::Path(path.clone());
        let envelope_target = EnvelopeTarget::from(target);

        assert_eq!(envelope_target.is_match(&path), true);
    }

    #[test]
    fn test_match_by_path_with_without_a_wildcard_returns_false() {
        let path = ActorPath::default().name("test");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let validated_path = ActorPath::default().name("not_matched");
        assert_eq!(envelope_target.is_match(&validated_path), false);
    }

    #[test]
    fn test_match_by_path_with_a_single_wildcard_in_the_beginning() {
        let path = ActorPath::default().name("*/processing/a");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path_1 = ActorPath::default().name("first/processing/a");
        let valid_path_2 = ActorPath::default().name("second/processing/a");
        let invalid_path = ActorPath::default().name("third/handling/a");
        assert_eq!(envelope_target.is_match(&valid_path_1), true);
        assert_eq!(envelope_target.is_match(&valid_path_2), true);
        assert_eq!(envelope_target.is_match(&invalid_path), false);
    }

    #[test]
    fn test_match_by_path_with_a_single_wildcard_in_the_middle() {
        let path = ActorPath::default().name("first/*/a");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path = ActorPath::default().name("first/processing/a");
        let invalid_path_1 = ActorPath::default().name("first/processing/b");
        let invalid_path_2 = ActorPath::default().name("first/processing/nested/a");
        let invalid_path_3 = ActorPath::default().name("second/handling/nested/a");
        assert_eq!(envelope_target.is_match(&valid_path), true);
        assert_eq!(envelope_target.is_match(&invalid_path_1), false);
        assert_eq!(envelope_target.is_match(&invalid_path_2), false);
        assert_eq!(envelope_target.is_match(&invalid_path_3), false);
    }

    #[test]
    fn test_match_by_path_with_a_single_wildcard_in_the_end() {
        let path = ActorPath::default().name("first/*");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path_1 = ActorPath::default().name("first/a");
        let valid_path_2 = ActorPath::default().name("first/processing/a");
        let valid_path_3 = ActorPath::default().name("first/processing/b");
        let valid_path_4 = ActorPath::default().name("first/processing/nested/a");
        let invalid_path_1 = ActorPath::default().name("second/a");
        let invalid_path_2 = ActorPath::default().name("second/nested/a");
        assert_eq!(envelope_target.is_match(&valid_path_1), true);
        assert_eq!(envelope_target.is_match(&valid_path_2), true);
        assert_eq!(envelope_target.is_match(&valid_path_3), true);
        assert_eq!(envelope_target.is_match(&valid_path_4), true);
        assert_eq!(envelope_target.is_match(&invalid_path_1), false);
        assert_eq!(envelope_target.is_match(&invalid_path_2), false);
    }

    #[test]
    fn test_match_by_path_with_a_single_and_limited_nesting_wildcard() {
        let path = ActorPath::default().name("first/*/");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path_1 = ActorPath::default().name("first/a/");
        let valid_path_2 = ActorPath::default().name("first/b/");
        let invalid_path_1 = ActorPath::default().name("first/handling/a");
        let invalid_path_2 = ActorPath::default().name("first/processing/b");
        let invalid_path_3 = ActorPath::default().name("first/processing/nested/a");
        let invalid_path_4 = ActorPath::default().name("second/a");
        let invalid_path_5 = ActorPath::default().name("second/nested/b");
        assert_eq!(envelope_target.is_match(&valid_path_1), true);
        assert_eq!(envelope_target.is_match(&valid_path_2), true);
        assert_eq!(envelope_target.is_match(&invalid_path_1), false);
        assert_eq!(envelope_target.is_match(&invalid_path_2), false);
        assert_eq!(envelope_target.is_match(&invalid_path_3), false);
        assert_eq!(envelope_target.is_match(&invalid_path_4), false);
        assert_eq!(envelope_target.is_match(&invalid_path_5), false);
    }

    #[test]
    fn test_match_by_path_with_multiple_wildcards() {
        let path = ActorPath::default().name("*/*/a");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path_1 = ActorPath::default().name("first/handling/a");
        let valid_path_2 = ActorPath::default().name("first/processing/a");
        let invalid_path_1 = ActorPath::default().name("first/processing/nested/a");
        let invalid_path_2 = ActorPath::default().name("second/a");
        let invalid_path_3 = ActorPath::default().name("second/nested/b");
        assert_eq!(envelope_target.is_match(&valid_path_1), true);
        assert_eq!(envelope_target.is_match(&valid_path_2), true);
        assert_eq!(envelope_target.is_match(&invalid_path_1), false);
        assert_eq!(envelope_target.is_match(&invalid_path_2), false);
        assert_eq!(envelope_target.is_match(&invalid_path_3), false);
    }

    #[test]
    fn test_match_by_path_with_a_single_wildcard_against_a_path_with_special_symbols() {
        let path = ActorPath::default().name("first/*/a");
        let target = Target::Path(path);
        let envelope_target = EnvelopeTarget::from(target);

        let valid_path_1 = ActorPath::default().name("first/hand.ling/a");
        let valid_path_2 = ActorPath::default().name("first/proce-ssing/a");
        let valid_path_3 = ActorPath::default().name("first/some_thing/a");
        let invalid_path_2 = ActorPath::default().name("second/a");
        let invalid_path_3 = ActorPath::default().name("second/nested/b");
        assert_eq!(envelope_target.is_match(&valid_path_1), true);
        assert_eq!(envelope_target.is_match(&valid_path_2), true);
        assert_eq!(envelope_target.is_match(&valid_path_3), true);
        assert_eq!(envelope_target.is_match(&invalid_path_2), false);
        assert_eq!(envelope_target.is_match(&invalid_path_3), false);
    }

    #[test]
    fn test_match_by_group_returns_false() {
        let target = Target::Group("test");
        let envelope_target = EnvelopeTarget::from(target);

        let actor_path = ActorPath::default();
        assert_eq!(envelope_target.is_match(&actor_path), false);
    }
}
