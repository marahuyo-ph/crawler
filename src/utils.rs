#[cfg(test)]
use std::process::{Child, Command};
#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
pub struct PythonServer {
    port: i16,
    child: Option<Child>,
}

#[cfg(test)]
impl PythonServer {
    pub fn new(port: i16) -> Self {
        Self { port, child: None }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
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

#[cfg(test)]
impl Drop for PythonServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
