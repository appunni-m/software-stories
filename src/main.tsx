import ReactDOM from 'react-dom/client';
import { App } from './App';
import { AppErrorBoundary } from './AppErrorBoundary';
import '@radix-ui/themes/styles.css';
import '@xyflow/react/dist/style.css';
import './App.css';
import './theme.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <AppErrorBoundary><App /></AppErrorBoundary>,
);
