import userEvent from '@testing-library/user-event';
import { expect, userEvent as ue } from '@storybook/test';

userEvent.click(document.body);

export const meta = { title: 'Multi|Rule' };

export const Primary = {
  play: () => {
    userEvent.click(document.body);
    expect(1).toBe(1);
  },
  name: 'Primary',
};

export const secondary = {};
