import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Ai from '../src/screens/Ai.svelte';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('shared AI credentials', () => {
  it('shows the key field when no key is saved', async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === 'settings') return { ai_provider: 'anthropic' };
      if (command === 'ai_key_status') {
        return { saved: false, masked: null, path: '/tmp/credentials.json', base_url: null };
      }
      return undefined;
    });

    render(Ai, {
      request: {
        input: '/mods/demo', from: '1.20.1', to: '1.21.1',
        loader: 'fabric', min_confidence: 0.8,
      },
      onStart: vi.fn(),
    });

    expect(await screen.findByLabelText('API key')).toBeInTheDocument();
  });

  it('saves the typed key before starting AI and clears the input', async () => {
    const onStart = vi.fn();
    invoke.mockImplementation(async (command: string) => {
      if (command === 'settings') return { ai_provider: 'anthropic' };
      if (command === 'ai_key_status') {
        return { saved: false, masked: null, path: '/tmp/credentials.json', base_url: null };
      }
      if (command === 'ai_key_save') return { masked: 'sk-ant-…0000', warning: null };
      if (command === 'start_ai_assist') return 'job-1';
      return undefined;
    });

    render(Ai, {
      request: {
        input: '/mods/demo', from: '1.20.1', to: '1.21.1',
        loader: 'fabric', min_confidence: 0.8,
      },
      onStart,
    });

    const input = await screen.findByLabelText('API key');
    await fireEvent.input(input, { target: { value: 'sk-ant-test-0000000000000000' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Start AI assist' }));

    const saveCall = invoke.mock.invocationCallOrder.findIndex((order) =>
      order === invoke.mock.invocationCallOrder[ invoke.mock.calls.findIndex(([cmd]) => cmd === 'ai_key_save') ],
    );
    const startCall = invoke.mock.invocationCallOrder[ invoke.mock.calls.findIndex(([cmd]) => cmd === 'start_ai_assist') ];
    expect(saveCall).toBeGreaterThanOrEqual(0);
    expect(startCall).toBeGreaterThan(invoke.mock.invocationCallOrder[ invoke.mock.calls.findIndex(([cmd]) => cmd === 'ai_key_save') ]);
    expect(onStart).toHaveBeenCalledWith('job-1');
    expect(input).toHaveValue('');
  });
});
