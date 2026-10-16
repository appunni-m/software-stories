import ReactDOM from 'react-dom/client';
import '@radix-ui/themes/styles.css';
import '@xyflow/react/dist/style.css';
import { WorkspacePreview } from './WorkspacePreview';
import './workspace-preview.css';

ReactDOM.createRoot(document.getElementById('root')!).render(<WorkspacePreview />);
