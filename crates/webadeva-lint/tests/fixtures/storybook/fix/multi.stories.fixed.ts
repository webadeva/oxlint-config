import { userEvent } from 'storybook/test';
import { expect, userEvent as ue } from '@storybook/test';

export default {}
await userEvent.click(document.body);

export const meta = { title: 'Multi|Rule' };

export const Primary = {
  play: async () => {
    await userEvent.click(document.body);
    await expect(1).toBe(1);
  },
  name: 'Primary',
};

export const secondary = {};
