use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Mattermost permanent link to the thread
    pub permalink: String,

    /// Custom issue title (optional)
    #[arg(long)]
    pub title: Option<String>,

    /// Mattermost server URL
    #[arg(long, env = "MATTERMOST_URL")]
    pub mm_url: String,

    /// Mattermost access token
    #[arg(long, env = "MATTERMOST_TOKEN")]
    pub mm_token: String,

    /// GitLab server URL
    #[arg(long, env = "GITLAB_URL")]
    pub gitlab_url: String,

    /// GitLab access token
    #[arg(long, env = "GITLAB_TOKEN")]
    pub gitlab_token: String,

    /// GitLab project ID
    #[arg(long, env = "GITLAB_PROJECT_ID")]
    pub project_id: String,

    /// Disable reply in Mattermost thread
    #[arg(long)]
    pub no_reply: bool,

    /// Skip preview and editor
    #[arg(long)]
    pub no_preview: bool,

    /// Modal
    #[arg(long, default_value = "deepseek-r1:latest")]
    pub ollama_model: String,
}
