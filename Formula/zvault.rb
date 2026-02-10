class Zvault < Formula
  desc "AI-native secrets manager — stop leaking secrets to LLMs"
  homepage "https://zvault.cloud"
  license any_of: ["MIT", "Apache-2.0"]
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/zvault/zvault/releases/download/v#{version}/zvault-v#{version}-darwin-aarch64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_intel do
      url "https://github.com/zvault/zvault/releases/download/v#{version}/zvault-v#{version}-darwin-x86_64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/zvault/zvault/releases/download/v#{version}/zvault-v#{version}-linux-aarch64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_intel do
      url "https://github.com/zvault/zvault/releases/download/v#{version}/zvault-v#{version}-linux-x86_64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "zvault"
    bin.install "vaultrs-server"
  end

  test do
    assert_match "zvault", shell_output("#{bin}/zvault --version")
  end
end
