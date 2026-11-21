import { userEvent } from '@storybook/test';
import { useState } from 'react';

export default { title: 'AwaitOff' };

export const Primary = {
  play: async () => {
    userEvent.click(document.body);
  },
};
