import { userEvent as ue, expect } from '@storybook/test';

export const meta = preview.meta({});
export default { title: 'Edge', excludeStories: ['a'] as const };

export const lower = {
  play: async () => {
    ue.click(document.body);
    userEvent.click(document.body);
    expect(1).toBe(1);
  },
};
export { default } from './other';
