# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "zvault-rails"
  spec.version       = "0.1.0"
  spec.authors       = ["ZVault"]
  spec.summary       = "ZVault integration for Ruby on Rails"
  spec.description   = "Load ZVault Cloud secrets into your Rails app. Replaces Rails.application.credentials."
  spec.homepage      = "https://zvault.cloud"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.1"

  spec.files         = ["lib/zvault/rails.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "railties", ">= 7.0"
end
