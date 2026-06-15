class Aphrodite < Formula
  desc "Generic LLM proxy with CCR + tool relay"
  homepage "https://github.com/PlayForm/Aphrodite"
  url "https://github.com/PlayForm/Aphrodite/releases/download/v0.4.1/aphrodite-macos-arm64"
  sha256 "0c3ccdc531a13d22682410f584031110074db0c7540c205fc0029b34abbef4a0"  # Update with: shasum -a 256 aphrodite-macos-arm64
  version "0.4.1"
  license "Apache-2.0"

  def install
    bin.install "aphrodite-macos-arm64" => "aphrodite"
  end

  def post_install
    (etc/"aphrodite").mkpath
    unless File.exist?(etc/"aphrodite/aphrodite.toml")
      (etc/"aphrodite/aphrodite.toml").write <<~EOS
        [defaults]
        api_url = "https://api.deepseek.com"
        model = "deepseek-v4-pro"

        [[proxies]]
        name = "cache"
        listen = "127.0.0.1:9797"
        mode = "cache"

        [[proxies]]
        name = "token"
        listen = "127.0.0.1:9798"
        mode = "token"
        tool_relay = true
      EOS
    end
  end

  test do
    system "#{bin}/aphrodite", "--version"
  end
end
