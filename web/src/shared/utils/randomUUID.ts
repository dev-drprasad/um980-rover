export function randomUUID() {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, function (c) {
    const r = (Math.random() * 16) | 0; // generate a random hex digit
    // for 'x', use the random digit; for 'y', adjust to adhere to UUID v4 specs
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}
