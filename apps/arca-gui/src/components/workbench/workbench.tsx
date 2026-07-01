import { WorkbenchLayout } from "./workbench-layout";
import { useWorkbenchModel } from "../../hooks/workbench/workbench-model";

export function Workbench() {
  const layout = useWorkbenchModel();
  return <WorkbenchLayout {...layout} />;
}
