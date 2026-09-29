import { describe, expect, it } from 'vitest';
import { isOllama } from './llmChat';

describe('local LLM endpoint classification', () => {
  it.each(['http://localhost:11434/v1', 'http://127.0.0.1:11434', 'http://[::1]:11434'])(
    'recognizes the actual loopback host in %s',
    (url) => expect(isOllama(url)).toBe(true),
  );

  it.each([
    'https://localhost.example.com/v1',
    'https://127.0.0.1.evil.example/v1',
    'https://api.example.com/localhost',
    'https://localhost@api.example.com/v1',
    'not-a-url-localhost',
  ])('does not classify a remote or invalid URL as local: %s', (url) => {
    expect(isOllama(url)).toBe(false);
  });
});
