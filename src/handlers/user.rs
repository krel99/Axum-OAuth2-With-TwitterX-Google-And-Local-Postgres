use axum::response::{Html, IntoResponse};

use crate::handlers::UserProfile;

pub async fn protected(user: UserProfile) -> Html<String> {
    let provider = if user.email.ends_with("@twitter.local") {
        "Twitter"
    } else {
        "Google"
    };

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Protected Area</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    padding: 20px;
                }}
                .container {{
                    max-width: 800px;
                    margin: 0 auto;
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                }}
                .info {{
                    background-color: #f0f8ff;
                    padding: 20px;
                    border-radius: 5px;
                    margin: 20px 0;
                }}
                .button {{
                    display: inline-block;
                    padding: 10px 20px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    margin: 10px;
                }}
                .button.logout {{
                    background-color: #dc3545;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Protected Area</h1>
                <div class="info">
                    <h2>Welcome!</h2>
                    <p>You are authenticated as: <strong>{}</strong></p>
                    <p>Provider: <strong>{}</strong></p>
                </div>
                <a href="/protected/profile" class="button">View Profile</a>
                <a href="/api/auth/logout" class="button logout">Logout</a>
            </div>
        </body>
        </html>
        "#,
        user.email, provider
    ))
}

pub async fn get_profile(user: UserProfile) -> impl IntoResponse {
    let (provider, display_name) = if user.email.ends_with("@twitter.local") {
        ("Twitter", user.email.replace("@twitter.local", ""))
    } else {
        ("Google", user.email.clone())
    };

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>User Profile</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    padding: 20px;
                }}
                .profile-card {{
                    max-width: 600px;
                    margin: 0 auto;
                    background: white;
                    padding: 30px;
                    border-radius: 10px;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                }}
                .button {{
                    display: inline-block;
                    padding: 10px 20px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    margin-top: 20px;
                }}
            </style>
        </head>
        <body>
            <div class="profile-card">
                <h2>User Profile</h2>
                <p><strong>Provider:</strong> {}</p>
                <p><strong>Display Name:</strong> {}</p>
                <p><strong>Email/ID:</strong> {}</p>
                <a href="/protected" class="button">Back to Protected Area</a>
            </div>
        </body>
        </html>
        "#,
        provider, display_name, user.email
    ))
}
