# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "zvault"
  spec.version       = "0.1.0"
  spec.authors       = ["ZVault"]
  spec.summary       = "Official Ruby SDK for ZVault Cloud secrets management"
  spec.description   = "Fetch and cache secrets from ZVault Cloud. Zero external dependencies."
  spec.homepage      = "https://zvault.cloud"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.1"

  spec.files         = ["lib/zvault.rb"]
  spec.require_paths = ["lib"]
end
