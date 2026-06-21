$task = @'
Fix all TypeScript compilation errors in the Canvas editor implementation. The Canvas files (CanvasEditor.vue, CanvasNode.vue, CanvasEdge.vue, useCanvas.ts) were created but have API misalignment issues. DO NOT create new files - only fix existing ones.

Errors to fix (by file):

## 1. src/composables/useCanvas.ts
- Add missing methods that CanvasEditor.vue needs:
  - updateNodePosition(id, x, y) - updates node position
  - startDraggingEdge(stepId, port, x, y) - begin edge drag
  - updateDraggingEdge(x, y) - update drag position  
  - finishDraggingEdge(targetStepId) - complete edge creation
  - cancelDraggingEdge() - cancel edge drag
  - selectNode(id) - set selectedNode
  - setPan(x, y) - set pan offset
  - setZoom(zoom) - set zoom level
  - resetView() - reset zoom/pan
  - removeEdge(id) - remove edge
- Export NodePosition type
- Remove unused runStates and store variables

## 2. src/types/types.ts
- Export NodePosition type: { x: number, y: number }

## 3. src/components/CanvasEditor.vue
- Fix duplicate import (remove one of the CanvasEditor imports)
- Fix CanvasEdge props: change from {x1,y1,x2,y2} to {from: {x,y}, to: {x,y}}
- Fix CanvasNode props: add position, nodeWidth, nodeHeight bindings
- Fix Map type access: use .get() instead of [] bracket notation OR cast properly
- Remove unused onMounted/onUnmounted imports
- Fix workflow prop type: accept null for a.workflow.value
- Fix all method calls to match useCanvas.ts exported names

## 4. src/components/CanvasEdge.vue  
- Accept props: from: {x,y}, to: {x,y} instead of x1,y1,x2,y2

## 5. src/components/CanvasNode.vue
- Import NodePosition from types
- Accept props: position (NodePosition), nodeWidth (number), nodeHeight (number)

## 6. src/pages/Editor.vue
- Fix duplicate CanvasEditor import (line 11-12)
- Fix @add-edge handler signature
- Fix CanvasEditor :workflow binding for null handling

## 7. src/stores/workflowStore.ts
- Remove unused Edge import

After fixing all errors, verify: npx vue-tsc --noEmit should exit with 0 errors.
'@
reasonix run --max-steps 60 $task 2>&1
