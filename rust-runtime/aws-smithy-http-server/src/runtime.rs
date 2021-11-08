use axum::extract::RequestParts;

#[derive(Debug, Clone, Copy, Default)]
pub struct AwsRestJson1<T>(pub T);

pub fn json_content_type<B>(req: &RequestParts<B>) -> Result<bool, http::StatusCode> {
    // TODO Replace this with a meaningful error.
    let rejection = http::StatusCode::from_u16(400).unwrap();

    let content_type = if let Some(content_type) = req
        .headers()
        .ok_or(rejection)?
        .get(http::header::CONTENT_TYPE)
    {
        content_type
    } else {
        return Ok(false);
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return Ok(false);
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return Ok(false);
    };

    let is_json_content_type = mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().filter(|name| *name == "json").is_some());

    Ok(is_json_content_type)
}
