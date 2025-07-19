use axum::response::Html;
use axum_messages::Messages;

use crate::utils::get_all_messages;

#[axum::debug_handler]
pub async fn newsletters_form(messages: Messages) -> Html<String> {
    let message = get_all_messages(messages);
    let idempotency_key = uuid::Uuid::new_v4();

    Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Publish newsletters</title>
  </head>
  <body>
    {message}
    <form action="/admin/newsletters" method="post">
      <label
        >Title
        <input type="text" placeholder="Enter the title" name="title" />
      </label>
      <br>
      <label
        >Html content
        <textarea placeholder="Enter html content" name="html_content"></textarea>
      </label>
      <br>
      <label
        >Text content
        <textarea placeholder="Enter text content" name="text_content"></textarea>
      </label>
      <br>
      <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
      <button type="submit">Publish</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
  </body>
</html>
            "#,
    ))
}
