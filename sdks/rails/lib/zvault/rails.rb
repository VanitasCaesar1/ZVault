# frozen_string_literal: true

require "net/http"
require "json"
require "uri"

module ZVault
  module Rails
    class Error < StandardError; end

    # ZVault Rails integration — replaces Rails.application.credentials.
    #
    # In config/initializers/zvault.rb:
    #   ZVault::Rails.load!(env: "production")
    #
    # Then access secrets:
    #   ZVault::Rails.secrets["DATABASE_URL"]
    #   ZVault::Rails["STRIPE_KEY"]
    #
    class << self
      attr_reader :secrets

      def load!(
        env: "production",
        token: nil,
        org_id: nil,
        project_id: nil,
        base_url: nil,
        inject_env: false
      )
        @token = token || ENV["ZVAULT_TOKEN"] || ""
        @org_id = org_id || ENV["ZVAULT_ORG_ID"] || ""
        @project_id = project_id || ENV["ZVAULT_PROJECT_ID"] || ""
        @base_url = (base_url || ENV["ZVAULT_URL"] || "https://api.zvault.cloud").chomp("/")

        if @token.empty? || @org_id.empty? || @project_id.empty?
          ::Rails.logger&.warn("[zvault] Missing config — skipping secret loading")
          @secrets = {}
          return
        end

        url = "#{@base_url}/v1/cloud/orgs/#{@org_id}/projects/#{@project_id}/envs/#{env}/secrets"
        @secrets = fetch_secrets(url)

        if inject_env
          @secrets.each do |k, v|
            ENV[k] = v unless ENV.key?(k)
          end
        end

        ::Rails.logger&.info("[zvault] Loaded #{@secrets.size} secrets from '#{env}'")
        @secrets
      rescue StandardError => e
        ::Rails.logger&.warn("[zvault] Failed to load secrets: #{e.message}")
        @secrets = {}
      end

      def [](key)
        (@secrets || {})[key]
      end

      private

      def fetch_secrets(url)
        uri = URI(url)
        http = Net::HTTP.new(uri.host, uri.port)
        http.use_ssl = uri.scheme == "https"
        http.open_timeout = 10
        http.read_timeout = 10

        req = Net::HTTP::Get.new(uri)
        req["Authorization"] = "Bearer #{@token}"
        req["Content-Type"] = "application/json"
        req["User-Agent"] = "zvault-rails/0.1.0"

        res = http.request(req)
        raise Error, "HTTP #{res.code}" unless res.is_a?(Net::HTTPSuccess)

        data = JSON.parse(res.body)
        result = {}
        Array(data["secrets"]).each do |s|
          result[s["key"]] = s["value"] if s["key"] && s["value"]
        end
        result.freeze
      end
    end
  end
end
