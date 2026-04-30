import { deepseekApiKey, deepseekBaseUrl } from '$lib/stores.svelte';
import { OpenAICompatibleClient } from './base';

/**
 * DeepSeek REST API client.
 *
 * https://api-docs.deepseek.com/
 */
export class DeepSeekClient extends OpenAICompatibleClient {
  constructor() {
    const apiKey = deepseekApiKey.current;
    if (!apiKey) {
      throw new Error('DeepSeek API key is not set');
    }
    const baseUrl = deepseekBaseUrl.current || 'https://api.deepseek.com/v1';
    super(baseUrl, apiKey);
  }
}
