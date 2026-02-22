defmodule ZVault.MixProject do
  use Mix.Project

  @version "0.1.0"
  @source_url "https://github.com/ArcadeLabsInc/zvault-elixir"

  def project do
    [
      app: :zvault,
      version: @version,
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "ZVault SDK for Elixir — fetch secrets from ZVault Cloud at runtime.",
      source_url: @source_url,
      docs: [main: "ZVault", source_ref: "v#{@version}"]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:httpoison, "~> 2.0"},
      {:jason, "~> 1.4"},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false}
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{"GitHub" => @source_url, "Docs" => "https://docs.zvault.cloud"},
      maintainers: ["ZVault Team"]
    ]
  end
end
