use crate::{commands::Commands, fetch::execute_fetch};

pub async fn execute_commands(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Fetch {
            url,
            user_agent,
            timeout,
            rate_limit,
            output_format,
        } => {
            execute_fetch(Commands::Fetch {
                url,
                user_agent,
                timeout,
                rate_limit,
                output_format,
            })
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::process::{Child, Command};
    use std::thread;
    use std::time::Duration;

    use clap::Parser;

    use crate::cli::execute_commands;
    use crate::commands::Cli;

    pub struct PythonServer {
        port: i16,
        child: Option<Child>,
    }

    impl PythonServer {
        fn new(port: i16) -> Self {
            Self { port, child: None }
        }

        fn run(&mut self) -> anyhow::Result<()> {
            let child = Command::new("python3")
                .arg("-m")
                .arg("http.server")
                .arg(self.port.to_string())
                .current_dir("test-site")
                .spawn()?;

            self.child = Some(child);
            thread::sleep(Duration::from_millis(500));
            Ok(())
        }
    }

    impl Drop for PythonServer {
        fn drop(&mut self) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    #[rstest::fixture]
    fn python_server() -> PythonServer {
      PythonServer::new(9001)
    }

    #[rstest::rstest]
    #[case("crawler fetch http://localhost:9001")]
    #[tokio::test]
    async fn test_cli(mut python_server: PythonServer,#[case] command:&str) -> anyhow::Result<()> {
      python_server.run()?;

      let args: Vec<&str> = command.split_whitespace().collect();
      let cli = Cli::parse_from(&args);

      if let Err(e) = execute_commands(cli.command).await {
        drop(python_server);
        return Err(e);
      }

      Ok(())
    }
}
