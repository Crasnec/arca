import type { WorkbenchLayoutProps } from "../../components/workbench";
import { buildWorkbenchLayoutModel } from "../../models/workbench-layout";
import { useWorkbenchWorkflows } from "./workbench-workflows";

export function useWorkbenchModel(): WorkbenchLayoutProps {
  return buildWorkbenchLayoutModel(useWorkbenchWorkflows());
}
