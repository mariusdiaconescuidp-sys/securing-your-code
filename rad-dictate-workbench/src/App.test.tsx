import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";
describe("Phase 0 transcript UI", () => {
  it("offers the required session controls", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "Hold to dictate" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear Session" })).toBeTruthy();
    expect(screen.getByLabelText("Editable transcript")).toBeTruthy();
  });
  it("clears an editable transcript", async () => {
    render(<App />);
    const transcript = screen.getByLabelText("Editable transcript") as HTMLTextAreaElement;
    fireEvent.change(transcript, { target: { value: "draft" } });
    fireEvent.click(screen.getByRole("button", { name: "Clear Session" }));
    await waitFor(() => expect(transcript.value).toBe(""));
  });
});
