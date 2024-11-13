use crate::context::{Context, Error};

#[poise::command(slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            show_subcommands: true,
            show_context_menu_commands: true,
            ephemeral: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}
