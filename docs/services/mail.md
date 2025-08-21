# Mail

Send emails with `elif-email`. Configure providers, render templates, and test with capture mode.

CLI
- Send a test email: `elifrs email send someone@example.com --subject "Hello" --body "Hi" --html`
- Render a template: `elifrs email template render welcome --context '{"name":"Ada"}' --format html`
- Provider config: `elifrs email provider configure sendgrid --interactive`
- Process email queue: `elifrs email queue process --limit 50 --timeout 30`
- Capture emails for tests: `elifrs email test capture --enable --dir tmp/emails`

Tips
- Prefer templates and pass context JSON for dynamic content.
- Use capture mode for integration testing without sending real emails.
