import type { Meta } from '@storybook/react';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fn } from '@storybook/vue3';
import { expect } from 'storybook/test';
import { Button } from '@storybook/react-vite';

export default { title: 'Imports' } satisfies Meta;

export const Primary = { play: async () => { await expect(1).toBe(1); } };
