import type { StorybookConfig } from '@storybook/react-vite';

const config: StorybookConfig = {
  stories: ['../src/**/*.stories.@(ts|tsx)'],
  addons: [
    '@storybook/addon-links',
    '@storybook/addon-essentials',
    '@storybook/addon-a11y/register.js',
    '@storybook/addon-docs/preset',
    '@storybook/addon-interactions/register',
    { name: '@storybook/addon-docs', options: {} },
    { name: 'missing-addon-in-object' },
    './local-addon',
    '/abs/addon',
    'C:\\addons\\win',
    'ignored-addon',
    '@storybook/addon-essentials',
  ],
  framework: '@storybook/react-vite',
};

export const addons = ['@storybook/addon-viewport', '@storybook/addon-links'];

const other: Record<string, unknown> = {};
other.cfg = { addons: ['assigned-missing', { name: 'react' }] };

export default config;
