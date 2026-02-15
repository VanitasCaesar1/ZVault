class Zvault < Formula
  desc "AI-native secrets manager — stop leaking secrets to LLMs"
  homepage "https://zvault.cloud"
  license any_of: ["MIT", "Apache-2.0"]
  version "0.1.1"
  head "https://github.com/VanitasCaesar1/zvault.git", branch: "main"

  url "https://static.crates.io/crates/zvault-cli/zvault-cli-0.1.1.crate"
  sha256 "09be3d37e3b3cf1fd2c483d9f3d46f778c1c5d0a85c2641caa6757c0af306d78"

  depends_on "rust" => :build

  def install
    if build.head?
      system "cargo", "build", "--release", "--package", "zvault-cli"
      system "cargo", "build", "--release", "--package", "zvault-server"
      bin.install "target/release/zvault"
      bin.install "target/release/zvault-server"
    else
      system "cargo", "install", "--root", prefix, "--path", "."
    end
  end

  def post_install
    (var/"zvault").mkpath
  end

  def caveats
    <<~EOS
      To get started:
        zvault status                        # Check vault health
        zvault init --shares 3 --threshold 2 # Initialize vault
        zvault import .env                   # Import your .env file

      AI Mode (Pro):
        zvault mcp-server                    # Start MCP server for AI assistants
        zvault setup cursor                  # Configure Cursor IDE
        zvault setup kiro                    # Configure Kiro IDE

      Data is stored in:
        #{var}/zvault

      Documentation:
        https://docs.zvault.cloud
    EOS
  end

  test do
    assert_match "zvault", shell_output("#{bin}/zvault --help")
  end
end
