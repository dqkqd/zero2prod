use axum::{Extension, response::Html};

use crate::authentication::CurrentUser;

#[axum::debug_handler]
pub async fn admin_dashboard(Extension(current_user): Extension<CurrentUser>) -> Html<String> {
    let username = current_user.username;
    Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Admin dashboard</title>
  </head>
  <body>
    <p> Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
      <li><a href="/admin/password">Change password</a></li>
      <li>
        <form name="logoutForm" action="/admin/logout" method="post">
          <input type="submit" value="Logout">
        </form>
      </li>
    </ol>
  </body>
</html>
"#
    ))
}
