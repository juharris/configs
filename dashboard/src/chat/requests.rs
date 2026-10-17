use crate::config::ChatCommandsConfig;

/// Classifies one incoming chat message by its leading configured command name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChatRequest {
    Help,
    Model(Option<String>),
    /// Carries a message that is not a dashboard command, including agent commands such as skills.
    Queue(String),
    Start(String),
    Status,
    Steer(String),
    Stop,
}

impl ChatRequest {
    pub fn parse(text: &str, commands: &ChatCommandsConfig) -> Self {
        // Leading whitespace lets people send a command that their chat client would otherwise intercept.
        let text = text.trim();
        let (name, argument) = text
            .split_once(char::is_whitespace)
            .map_or((text, ""), |(name, argument)| (name, argument.trim()));
        if name == commands.help.name {
            Self::Help
        } else if name == commands.model.name {
            Self::Model((!argument.is_empty()).then(|| argument.to_owned()))
        } else if name == commands.start.name {
            Self::Start(argument.to_owned())
        } else if name == commands.status.name {
            Self::Status
        } else if name == commands.steer.name {
            Self::Steer(argument.to_owned())
        } else if name == commands.stop.name {
            Self::Stop
        } else {
            Self::Queue(text.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChatRequest;
    use crate::chat::tests::chat_config;

    #[test]
    fn parses_configured_commands_and_their_arguments() {
        let commands = chat_config().commands;

        assert_eq!(
            ChatRequest::parse("/ai fix the build\nthen push", &commands),
            ChatRequest::Start("fix the build\nthen push".to_owned())
        );
        assert_eq!(
            ChatRequest::parse(" /steer  use the other API ", &commands),
            ChatRequest::Steer("use the other API".to_owned())
        );
        assert_eq!(
            ChatRequest::parse("/model sonnet", &commands),
            ChatRequest::Model(Some("sonnet".to_owned()))
        );
        assert_eq!(
            ChatRequest::parse("/model", &commands),
            ChatRequest::Model(None)
        );
        assert_eq!(ChatRequest::parse("/help", &commands), ChatRequest::Help);
        assert_eq!(
            ChatRequest::parse("/status", &commands),
            ChatRequest::Status
        );
        assert_eq!(
            ChatRequest::parse("/stop now", &commands),
            ChatRequest::Stop
        );
        assert_eq!(
            ChatRequest::parse("/ai", &commands),
            ChatRequest::Start(String::new())
        );
    }

    #[test]
    fn queues_plain_text_and_unknown_commands_for_the_agent() {
        let commands = chat_config().commands;

        assert_eq!(
            ChatRequest::parse("looks good, ship it", &commands),
            ChatRequest::Queue("looks good, ship it".to_owned())
        );
        assert_eq!(
            ChatRequest::parse("/review the diff", &commands),
            ChatRequest::Queue("/review the diff".to_owned())
        );
        assert_eq!(
            ChatRequest::parse("/aim higher", &commands),
            ChatRequest::Queue("/aim higher".to_owned())
        );
    }
}
