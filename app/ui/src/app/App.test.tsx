import { render, screen } from "@testing-library/react";
import { App } from "./App";
import { MockBackend } from "../backend/mock";

// req: UI-2
test("renders the welcome screen with the mock backend", async () => {
  render(<App backend={new MockBackend()} />);
  expect(await screen.findByText("Confidence")).toBeInTheDocument();
  expect(await screen.findByTestId("backend-kind")).toHaveTextContent("mock backend");
});
