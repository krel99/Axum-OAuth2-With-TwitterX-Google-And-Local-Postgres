use axum::response::Html;
use axum::Extension;

use crate::oauth::ClientIds;

pub async fn homepage(Extension(client_ids): Extension<ClientIds>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>OAuth2 Demo</title>
            <style>
                body {{
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
                    margin: 0;
                    padding: 0;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }}
                .container {{
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                    text-align: center;
                    max-width: 500px;
                    width: 100%;
                }}
                h1 {{
                    color: #333;
                    margin-bottom: 10px;
                    font-size: 32px;
                }}
                .subtitle {{
                    color: #666;
                    margin-bottom: 30px;
                    font-size: 18px;
                }}
                .button-group {{
                    display: flex;
                    gap: 15px;
                    margin-bottom: 20px;
                }}
                .button {{
                    display: inline-flex;
                    align-items: center;
                    justify-content: center;
                    padding: 12px 24px;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-weight: 500;
                    transition: all 0.3s ease;
                    flex: 1;
                }}
                .button.google {{
                    background-color: #4285f4;
                }}
                .button.google:hover {{
                    background-color: #357ae8;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(66, 133, 244, 0.3);
                }}
                .button.twitter {{
                    background-color: #1DA1F2;
                }}
                .button.twitter:hover {{
                    background-color: #1a91da;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(29, 161, 242, 0.3);
                }}
                .button.protected {{
                    background-color: #667eea;
                    margin-top: 10px;
                }}
                .button.protected:hover {{
                    background-color: #5a67d8;
                    transform: translateY(-2px);
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>🔐 OAuth Demo</h1>
                <p class="subtitle">Secure OAuth2 authentication with Google and Twitter</p>

                <div class="button-group">
                    <a href="https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={}&response_type=code&redirect_uri=http://localhost:8000/api/auth/google_callback"
                       class="button google">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
                        </svg>
                        Google
                    </a>

                    <a href="/api/auth/twitter_login"
                       class="button twitter">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M23.643 4.937c-.835.37-1.732.62-2.675.733.962-.576 1.7-1.49 2.048-2.578-.9.534-1.897.922-2.958 1.13-.85-.904-2.06-1.47-3.4-1.47-2.572 0-4.658 2.086-4.658 4.66 0 .364.042.718.12 1.06-3.873-.195-7.304-2.05-9.602-4.868-.4.69-.63 1.49-.63 2.342 0 1.616.823 3.043 2.072 3.878-.764-.025-1.482-.234-2.11-.583v.06c0 2.257 1.605 4.14 3.737 4.568-.392.106-.803.162-1.227.162-.3 0-.593-.028-.877-.082.593 1.85 2.313 3.198 4.352 3.234-1.595 1.25-3.604 1.995-5.786 1.995-.376 0-.747-.022-1.112-.065 2.062 1.323 4.51 2.093 7.14 2.093 8.57 0 13.255-7.098 13.255-13.254 0-.2-.005-.402-.014-.602.91-.658 1.7-1.477 2.323-2.41z"/>
                        </svg>
                        Twitter
                    </a>
                </div>

                <a href="/protected" class="button protected">🔒 Access Protected Area</a>
            </div>
        </body>
        </html>
        "#,
        client_ids.google
    ))
}

pub async fn login_page(Extension(client_ids): Extension<ClientIds>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Login - OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }}
                .login-container {{
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                    text-align: center;
                    max-width: 500px;
                }}
                .oauth-button {{
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    padding: 12px 24px;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-size: 16px;
                    font-weight: 500;
                    margin: 15px 0;
                    transition: all 0.3s ease;
                }}
                .google-button {{
                    background-color: #4285f4;
                }}
                .google-button:hover {{
                    background-color: #357ae8;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(66, 133, 244, 0.3);
                }}
                .twitter-button {{
                    background-color: #1DA1F2;
                }}
                .twitter-button:hover {{
                    background-color: #1a91da;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(29, 161, 242, 0.3);
                }}
            </style>
        </head>
        <body>
            <div class="login-container">
                <h1>Login Required</h1>
                <p>Please authenticate with one of the following providers:</p>

                <a href="https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={}&response_type=code&redirect_uri=http://localhost:8000/api/auth/google_callback"
                   class="oauth-button google-button">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                        <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                        <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                        <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                        <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
                    </svg>
                    Sign in with Google
                </a>

                <a href="/api/auth/twitter_login"
                   class="oauth-button twitter-button">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                        <path d="M23.643 4.937c-.835.37-1.732.62-2.675.733.962-.576 1.7-1.49 2.048-2.578-.9.534-1.897.922-2.958 1.13-.85-.904-2.06-1.47-3.4-1.47-2.572 0-4.658 2.086-4.658 4.66 0 .364.042.718.12 1.06-3.873-.195-7.304-2.05-9.602-4.868-.4.69-.63 1.49-.63 2.342 0 1.616.823 3.043 2.072 3.878-.764-.025-1.482-.234-2.11-.583v.06c0 2.257 1.605 4.14 3.737 4.568-.392.106-.803.162-1.227.162-.3 0-.593-.028-.877-.082.593 1.85 2.313 3.198 4.352 3.234-1.595 1.25-3.604 1.995-5.786 1.995-.376 0-.747-.022-1.112-.065 2.062 1.323 4.51 2.093 7.14 2.093 8.57 0 13.255-7.098 13.255-13.254 0-.2-.005-.402-.014-.602.91-.658 1.7-1.477 2.323-2.41z"/>
                    </svg>
                    Sign in with Twitter
                </a>
            </div>
        </body>
        </html>
        "#,
        client_ids.google
    ))
}
