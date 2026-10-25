import {Button,Dialog,Flex,Table} from '@radix-ui/themes';

const shortcuts=[
  ['Guide','⌘1'],['Files','⌘2'],['Changes','⌘3'],['Find a file or folder','⌘K'],
  ['Open a repository','⌘O'],['Back','⌘['],['Forward','⌘]'],['Zoom in / out (20% steps)','⌘= / ⌘−'],
  ['Actual size (100%)','⌘0'],
];

/** Help is an explicit view, not a saved background outcome. */
export function KeyboardShortcuts({open,onOpenChange}:{open:boolean;onOpenChange:(open:boolean)=>void}) {
  return <Dialog.Root open={open} onOpenChange={onOpenChange}>
    <Dialog.Content maxWidth="520px" onCloseAutoFocus={event=>{
      event.preventDefault();document.querySelector<HTMLElement>('[data-focus-key="tools-button"]')?.focus();
    }}>
      <Dialog.Title>Keyboard shortcuts</Dialog.Title>
      <Dialog.Description size="2" mb="4">Navigate the workspace, open a repository, and adjust the view.</Dialog.Description>
      <Table.Root variant="surface">
        <Table.Header><Table.Row><Table.ColumnHeaderCell>Action</Table.ColumnHeaderCell><Table.ColumnHeaderCell>Shortcut</Table.ColumnHeaderCell></Table.Row></Table.Header>
        <Table.Body>{shortcuts.map(([action,keys])=><Table.Row key={action}><Table.RowHeaderCell>{action}</Table.RowHeaderCell><Table.Cell><kbd>{keys}</kbd></Table.Cell></Table.Row>)}</Table.Body>
      </Table.Root>
      <Flex justify="end" mt="4"><Dialog.Close><Button>Done</Button></Dialog.Close></Flex>
    </Dialog.Content>
  </Dialog.Root>;
}
