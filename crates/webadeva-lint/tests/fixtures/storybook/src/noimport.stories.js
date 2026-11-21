export default { title: 'NoImport' };

export const Primary = {
  play: async ({ canvasElement }) => {
    userEvent.click(canvasElement);
    expect(canvasElement).toBeTruthy();
    (expect)(1);
  },
};
