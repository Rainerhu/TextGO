import { anthropicApiKey, anthropicBaseUrl } from '$lib/stores.svelte';
import { OpenAICompatibleClient } from './base';

/**
 * Anthropic REST API client.
 *
 * https://platform.claude.com/docs/en/api/openai-sdk
 */
export class AnthropicClient extends OpenAICompatibleClient {
  constructor() {
    const apiKey = anthropicApiKey.current;
    if (!apiKey) {
      throw new Error('Anthropic API key is not set');
    }
    const baseUrl = anthropicBaseUrl.current || 'https://api.anthropic.com/v1';
    super(baseUrl, apiKey);
  }
}
