use telegram_bot::RawMessageEntity;

use crate::data::FCPInfo;

pub const MSG_FORMAT: usize = 1;

pub fn format_msg(info: &FCPInfo) -> (String, Vec<RawMessageEntity>) {
    let mut utf16_cnt = 0;
    let mut string = String::new();
    let mut tokens = Vec::new();

    for (s, t) in format_msg_tokens(info) {
        string.push_str(&s);
        let utf16_len = str::encode_utf16(&s).count() as i64;

        if let Some(t) = t {
            tokens.push(t.entity(utf16_cnt, utf16_len));
        }

        utf16_cnt += utf16_len;
    }

    (string, tokens)
}

#[derive(Clone)]
enum TokenMeta {
    Bold,
    Url,
    TextLink(String),
    Hashtag,
}

impl TokenMeta {
    pub fn entity(self, offset: i64, length: i64) -> RawMessageEntity {
        use TokenMeta::*;
        let (type_, url) = match self {
            Bold => ("bold", None),
            Url => ("url", None),
            TextLink(u) => ("text_link", Some(u)),
            Hashtag => ("hashtag", None),
        };
        RawMessageEntity {
            type_: type_.to_owned(),
            offset, length,
            url,
            user: None,
        }
    }
}

fn format_msg_tokens(info: &FCPInfo) -> Vec<(String, Option<TokenMeta>)> {
    let link = format!(
        "https://github.com/{}/{}/{}",
        info.repo,
        if info.is_pr { "pull" } else { "issues" },
        info.issue
    );
    let mut tokens = vec![
        (info.title.clone(), Some(TokenMeta::Bold)),
        ("\n".to_owned(), None),
        (link, Some(TokenMeta::Url)),
        ("\n\n".to_owned(), None),
    ];

    let mut tag_t: Vec<_> = info.tags.iter().filter(|e| e.starts_with("T-")).collect();
    let mut tag_cat: Vec<_> = info.tags.iter().filter(|e| e.as_bytes()[1] == b'-' && e.as_bytes()[0] != b'T').collect();
    let mut tag_others: Vec<_> = info.tags.iter().filter(|e| e.as_bytes()[1] != b'-').collect();
    tag_t.sort();
    tag_cat.sort();
    tag_others.sort();

    tokens.extend(tag_t
        .iter()
        .map(|e| format!("#{}", e.replace("-", "_")))
        .map(|e| (e, Some(TokenMeta::Hashtag)))
        .intersperse((" ".to_owned(), None)));
    
    if tag_t.len() > 0 {
        tokens.push(("\n".to_owned(), None));
    }

    tokens.extend(tag_cat
        .iter()
        .map(|e| format!("#{}", e.replace("-", "_")))
        .map(|e| (e, Some(TokenMeta::Hashtag)))
        .intersperse((" ".to_owned(), None)));

    if tag_cat.len() > 0 {
        tokens.push(("\n".to_owned(), None));
    }

    tokens.extend(tag_others
        .iter()
        .map(|e| format!("#{}", e.replace("-", "_")))
        .map(|e| (e, Some(TokenMeta::Hashtag)))
        .intersperse((" ".to_owned(), None)));

    if tag_others.len() > 0 {
        tokens.push(("\n".to_owned(), None));
    }

    tokens.push(("--------\n⏳ ".to_owned(), None));

    tokens.extend(info.pending
        .iter()
        .map(|e| (format!("@{}", e), format!("https://github.com/{}", e)))
        .map(|(un, link)| (un, Some(TokenMeta::TextLink(link))))
        .intersperse((" ".to_owned(), None)));

    tokens.push(("\n✅ ".to_owned(), None));

    tokens.extend(info.approved
        .iter()
        .map(|e| format!("@{}", e))
        .map(|un| (un, None))
        .intersperse((" ".to_owned(), None)));

    tokens.push(("\n".to_owned(), None));
    tokens.push(("\u{200B}".repeat(MSG_FORMAT), None));
    tokens
}