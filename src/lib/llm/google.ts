import { geminiApiKey, geminiBaseUrl } from '$lib/stores.svelte';
import { OpenAICompatibleClient } from './base';

/**
 * Google Gemini REST API client.
 *
 * https://ai.google.dev/gemini-api/docs/openai
 */
export class GeminiClient extends OpenAICompatibleClient {
  constructor() {
    const apiKey = geminiApiKey.current;
    if (!apiKey) {
      throw new Error('Google Gemini API key is not set');
    }
    const baseUrl = geminiBaseUrl.current || 'https://generativelanguage.googleapis.com/v1beta/openai';
    super(baseUrl, apiKey);
  }
}
