use std::str::FromStr;

pub fn prompt_text_handle_errors<T>(prompt: &str) -> eyre::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error,
{
    loop {
        let t = inquire::Text::new(prompt).prompt()?;

        match t.trim().parse() {
            Ok(t) => return Ok(t),
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
}

pub fn prompt_text_skippable_handle_errors<T>(
    prompt: &str,
) -> eyre::Result<Option<T>>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error,
{
    loop {
        let t = inquire::Text::new(prompt).prompt_skippable()?;

        let Some(t) = t else {
            return Ok(None);
        };

        match t.trim().parse() {
            Ok(t) => return Ok(Some(t)),
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
}
