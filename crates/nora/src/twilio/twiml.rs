//! TwiML (Twilio Markup Language) builder for generating voice responses
//!
//! Creates XML responses that Twilio uses to control phone calls.

use std::fmt::Write;

/// Builder for generating TwiML responses
#[derive(Debug, Clone, Default)]
pub struct TwimlBuilder {
    elements: Vec<TwimlElement>,
}

/// TwiML elements
#[derive(Debug, Clone)]
enum TwimlElement {
    Say {
        text: String,
        voice: String,
        language: String,
    },
    Gather {
        input: GatherInput,
        action: String,
        method: String,
        timeout: u32,
        speech_timeout: String,
        speech_model: String,
        language: String,
        hints: Option<String>,
        #[allow(dead_code)]
        partial_result_callback: Option<String>,
        children: Vec<TwimlElement>,
    },
    Play {
        url: String,
        loop_count: u32,
    },
    Pause {
        length: u32,
    },
    Record {
        action: String,
        method: String,
        max_length: u32,
        transcribe: bool,
        play_beep: bool,
    },
    Redirect {
        url: String,
        method: String,
    },
    Hangup,
    Reject {
        reason: String,
    },
}

/// Input types for Gather
#[derive(Debug, Clone)]
pub enum GatherInput {
    Speech,
    Dtmf,
    SpeechDtmf,
}

impl GatherInput {
    fn as_str(&self) -> &str {
        match self {
            GatherInput::Speech => "speech",
            GatherInput::Dtmf => "dtmf",
            GatherInput::SpeechDtmf => "speech dtmf",
        }
    }
}

impl TwimlBuilder {
    /// Create a new TwiML builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a Say element (text-to-speech)
    pub fn say(mut self, text: &str, voice: &str, language: &str) -> Self {
        self.elements.push(TwimlElement::Say {
            text: xml_escape(text),
            voice: voice.to_string(),
            language: language.to_string(),
        });
        self
    }

    /// Add a Say element with default British voice
    pub fn say_british(mut self, text: &str) -> Self {
        self.elements.push(TwimlElement::Say {
            text: xml_escape(text),
            voice: "Polly.Amy".to_string(),
            language: "en-GB".to_string(),
        });
        self
    }

    /// Add a Gather element for collecting speech input
    pub fn gather_speech(
        mut self,
        action: &str,
        timeout: u32,
        language: &str,
        hints: Option<&str>,
        prompt: Option<&str>,
    ) -> Self {
        let mut children = Vec::new();
        if let Some(prompt_text) = prompt {
            children.push(TwimlElement::Say {
                text: xml_escape(prompt_text),
                voice: "Polly.Amy".to_string(),
                language: language.to_string(),
            });
        }

        self.elements.push(TwimlElement::Gather {
            input: GatherInput::Speech,
            action: action.to_string(),
            method: "POST".to_string(),
            timeout,
            speech_timeout: "auto".to_string(),
            speech_model: "phone_call".to_string(),
            language: language.to_string(),
            hints: hints.map(|s| s.to_string()),
            partial_result_callback: None,
            children,
        });
        self
    }

    /// Add a Gather element with DTMF and speech
    pub fn gather_speech_dtmf(
        mut self,
        action: &str,
        timeout: u32,
        language: &str,
        prompt: Option<&str>,
    ) -> Self {
        let mut children = Vec::new();
        if let Some(prompt_text) = prompt {
            children.push(TwimlElement::Say {
                text: xml_escape(prompt_text),
                voice: "Polly.Amy".to_string(),
                language: language.to_string(),
            });
        }

        self.elements.push(TwimlElement::Gather {
            input: GatherInput::SpeechDtmf,
            action: action.to_string(),
            method: "POST".to_string(),
            timeout,
            speech_timeout: "auto".to_string(),
            speech_model: "phone_call".to_string(),
            language: language.to_string(),
            hints: None,
            partial_result_callback: None,
            children,
        });
        self
    }

    /// Add a Gather element with audio playback (using NORA's TTS)
    ///
    /// Instead of using <Say>, this plays pre-generated audio from NORA's voice engine
    pub fn gather_speech_with_audio(
        mut self,
        audio_url: &str,
        action: &str,
        timeout: u32,
        language: &str,
        hints: Option<&str>,
    ) -> Self {
        let children = vec![TwimlElement::Play {
            url: audio_url.to_string(),
            loop_count: 1,
        }];

        self.elements.push(TwimlElement::Gather {
            input: GatherInput::Speech,
            action: action.to_string(),
            method: "POST".to_string(),
            timeout,
            speech_timeout: "auto".to_string(),
            speech_model: "phone_call".to_string(),
            language: language.to_string(),
            hints: hints.map(|s| s.to_string()),
            partial_result_callback: None,
            children,
        });
        self
    }

    /// Add a Play element (play audio file)
    pub fn play(mut self, url: &str, loop_count: u32) -> Self {
        self.elements.push(TwimlElement::Play {
            url: url.to_string(),
            loop_count,
        });
        self
    }

    /// Add a Pause element
    pub fn pause(mut self, seconds: u32) -> Self {
        self.elements.push(TwimlElement::Pause { length: seconds });
        self
    }

    /// Add a Record element
    pub fn record(
        mut self,
        action: &str,
        max_length: u32,
        transcribe: bool,
        play_beep: bool,
    ) -> Self {
        self.elements.push(TwimlElement::Record {
            action: action.to_string(),
            method: "POST".to_string(),
            max_length,
            transcribe,
            play_beep,
        });
        self
    }

    /// Add a Redirect element
    pub fn redirect(mut self, url: &str) -> Self {
        self.elements.push(TwimlElement::Redirect {
            url: url.to_string(),
            method: "POST".to_string(),
        });
        self
    }

    /// Add a Hangup element
    pub fn hangup(mut self) -> Self {
        self.elements.push(TwimlElement::Hangup);
        self
    }

    /// Add a Reject element
    pub fn reject(mut self, reason: &str) -> Self {
        self.elements.push(TwimlElement::Reject {
            reason: reason.to_string(),
        });
        self
    }

    /// Build the TwiML XML string
    pub fn build(self) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Response>\n");

        for element in self.elements {
            render_element(&mut xml, &element, 1);
        }

        xml.push_str("</Response>");
        xml
    }

    /// Create a greeting response with audio playback and speech gathering
    ///
    /// Uses NORA's voice engine for TTS instead of Twilio's Polly
    pub fn greeting_with_audio_and_gather(
        audio_url: &str,
        gather_action: &str,
        language: &str,
    ) -> String {
        TwimlBuilder::new()
            .gather_speech_with_audio(audio_url, gather_action, 10, language, None)
            .say_british(
                "I didn't catch that. Please try again, or press any key to speak with me.",
            )
            .redirect(gather_action)
            .build()
    }

    /// Create a simple response with audio playback and continue gathering
    ///
    /// Uses NORA's voice engine for TTS instead of Twilio's Polly
    pub fn respond_with_audio_and_gather(
        audio_url: &str,
        gather_action: &str,
        language: &str,
    ) -> String {
        TwimlBuilder::new()
            .gather_speech_with_audio(audio_url, gather_action, 10, language, None)
            .say_british("Is there anything else I can help you with?")
            .redirect(gather_action)
            .build()
    }

    /// Create a goodbye response with audio playback
    ///
    /// Uses NORA's voice engine for TTS instead of Twilio's Polly
    pub fn goodbye_with_audio(audio_url: &str) -> String {
        TwimlBuilder::new()
            .play(audio_url, 1)
            .pause(1)
            .hangup()
            .build()
    }

    /// Create a greeting response with speech gathering
    pub fn greeting_with_gather(greeting: &str, action: &str, language: &str) -> String {
        TwimlBuilder::new()
            .gather_speech(action, 10, language, None, Some(greeting))
            .say_british(
                "I didn't catch that. Please try again, or press any key to speak with me.",
            )
            .redirect(action)
            .build()
    }

    /// Create a simple response and continue gathering
    pub fn respond_and_gather(response: &str, action: &str, language: &str) -> String {
        TwimlBuilder::new()
            .gather_speech(action, 10, language, None, Some(response))
            .say_british("Is there anything else I can help you with?")
            .redirect(action)
            .build()
    }

    /// Create a goodbye response
    pub fn goodbye(message: &str) -> String {
        TwimlBuilder::new()
            .say_british(message)
            .pause(1)
            .hangup()
            .build()
    }

    /// Create an error response
    pub fn error(message: &str, retry_url: Option<&str>) -> String {
        let mut builder = TwimlBuilder::new().say_british(message).pause(1);

        if let Some(url) = retry_url {
            builder = builder.redirect(url);
        } else {
            builder = builder.hangup();
        }

        builder.build()
    }
}

/// Render a TwiML element to XML
fn render_element(xml: &mut String, element: &TwimlElement, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match element {
        TwimlElement::Say {
            text,
            voice,
            language,
        } => {
            let _ = writeln!(
                xml,
                "{}<Say voice=\"{}\" language=\"{}\">{}</Say>",
                indent_str, voice, language, text
            );
        }
        TwimlElement::Gather {
            input,
            action,
            method,
            timeout,
            speech_timeout,
            speech_model,
            language,
            hints,
            partial_result_callback: _,
            children,
        } => {
            let _ = write!(
                xml,
                "{}<Gather input=\"{}\" action=\"{}\" method=\"{}\" timeout=\"{}\" \
                 speechTimeout=\"{}\" speechModel=\"{}\" language=\"{}\"",
                indent_str,
                input.as_str(),
                action,
                method,
                timeout,
                speech_timeout,
                speech_model,
                language
            );

            if let Some(h) = hints {
                let _ = write!(xml, " hints=\"{}\"", h);
            }

            if children.is_empty() {
                let _ = writeln!(xml, "/>");
            } else {
                let _ = writeln!(xml, ">");
                for child in children {
                    render_element(xml, child, indent + 1);
                }
                let _ = writeln!(xml, "{}</Gather>", indent_str);
            }
        }
        TwimlElement::Play { url, loop_count } => {
            let _ = writeln!(
                xml,
                "{}<Play loop=\"{}\">{}</Play>",
                indent_str, loop_count, url
            );
        }
        TwimlElement::Pause { length } => {
            let _ = writeln!(xml, "{}<Pause length=\"{}\"/>", indent_str, length);
        }
        TwimlElement::Record {
            action,
            method,
            max_length,
            transcribe,
            play_beep,
        } => {
            let _ = writeln!(
                xml,
                "{}<Record action=\"{}\" method=\"{}\" maxLength=\"{}\" \
                 transcribe=\"{}\" playBeep=\"{}\"/>",
                indent_str, action, method, max_length, transcribe, play_beep
            );
        }
        TwimlElement::Redirect { url, method } => {
            let _ = writeln!(
                xml,
                "{}<Redirect method=\"{}\">{}</Redirect>",
                indent_str, method, url
            );
        }
        TwimlElement::Hangup => {
            let _ = writeln!(xml, "{}<Hangup/>", indent_str);
        }
        TwimlElement::Reject { reason } => {
            let _ = writeln!(xml, "{}<Reject reason=\"{}\"/>", indent_str, reason);
        }
    }
}

/// Escape special XML characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_twiml() {
        let twiml = TwimlBuilder::greeting_with_gather(
            "Hello, this is Nora. How may I help you?",
            "/api/twilio/speech",
            "en-GB",
        );
        assert!(twiml.contains("<Response>"));
        assert!(twiml.contains("<Gather"));
        assert!(twiml.contains("Nora"));
        assert!(twiml.contains("</Response>"));
    }

    #[test]
    fn test_goodbye_twiml() {
        let twiml = TwimlBuilder::goodbye("Thank you for calling. Goodbye!");
        assert!(twiml.contains("<Hangup/>"));
        assert!(twiml.contains("Goodbye"));
    }

    #[test]
    fn test_xml_escape() {
        let escaped = xml_escape("Hello <world> & \"friends\"");
        assert_eq!(escaped, "Hello &lt;world&gt; &amp; &quot;friends&quot;");
    }
}
