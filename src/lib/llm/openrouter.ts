import { openrouterApiKey, openrouterBaseUrl } from '$lib/stores.svelte';
import { OpenAICompatibleClient } from './base';

/**
 * OpenRouter REST API client.
 *
 * https://openrouter.ai/docs/quickstart#using-the-openai-sdk
 */
export class OpenRouterClient extends OpenAICompatibleClient {
  constructor() {
    const apiKey = openrouterApiKey.current;
    if (!apiKey) {
      throw new Error('OpenRouter API key is not set');
    }
    const baseUrl = openrouterBaseUrl.current || 'https://openrouter.ai/api/v1';
    super(baseUrl, apiKey);
  }
}
