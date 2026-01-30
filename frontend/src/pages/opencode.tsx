import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Terminal,
  ExternalLink,
  Check,
  X,
  Zap,
  Shield,
  DollarSign,
  Layers,
  AlertCircle,
} from 'lucide-react';

export function OpenCodePage() {
  const strengths = [
    {
      icon: Layers,
      title: 'Model Flexibility',
      description: 'Supports 75+ providers including Claude, OpenAI, Gemini, and local models via Ollama',
    },
    {
      icon: DollarSign,
      title: 'Cost Effective',
      description: 'Free and open source (MIT license) - pay only your AI provider directly',
    },
    {
      icon: Shield,
      title: 'Privacy First',
      description: 'No code or context data stored - suitable for regulated environments',
    },
    {
      icon: Terminal,
      title: 'Client/Server Architecture',
      description: 'Enables remote control, persistent workspaces, and Docker container sessions',
    },
  ];

  const limitations = [
    'No checkpoint/rewind system like Claude Code',
    'Single-agent architecture (no parallel subagents)',
    'Known stability issues and Windows problems',
    'Cannot copy/paste from conversations',
    'Cannot queue up multiple requests',
  ];

  const comparisonData = [
    { feature: 'Source Code', opencode: 'MIT Open Source', claudecode: 'Proprietary' },
    { feature: 'Model Support', opencode: '75+ providers + local', claudecode: 'Claude only' },
    { feature: 'Cost', opencode: 'Free + BYOK', claudecode: '$20-200/month' },
    { feature: 'Checkpoint/Rewind', opencode: 'No', claudecode: 'Yes' },
    { feature: 'Subagents', opencode: 'No', claudecode: 'Yes' },
    { feature: 'Data Storage', opencode: 'None', claudecode: 'Local sessions' },
  ];

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-lg">
              <Terminal className="h-6 w-6 text-green-600" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-2xl font-bold">OpenCode</h1>
                <Badge variant="outline" className="text-amber-600 border-amber-300">
                  Not Configured
                </Badge>
              </div>
              <p className="text-muted-foreground text-sm mt-1">
                Open-source AI coding agent with multi-provider support
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" asChild>
              <a
                href="https://opencode.ai"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-2"
              >
                <ExternalLink className="h-4 w-4" />
                Documentation
              </a>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <a
                href="https://github.com/sst/opencode"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-2"
              >
                <ExternalLink className="h-4 w-4" />
                GitHub
              </a>
            </Button>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-auto p-6">
        {/* Not Configured Banner */}
        <Card className="mb-6 border-amber-200 bg-amber-50 dark:bg-amber-950/20 dark:border-amber-900">
          <CardContent className="flex items-center gap-4 py-4">
            <AlertCircle className="h-8 w-8 text-amber-600" />
            <div className="flex-1">
              <h3 className="font-semibold text-amber-900 dark:text-amber-100">
                OpenCode is not configured for any agents
              </h3>
              <p className="text-sm text-amber-700 dark:text-amber-300">
                This tool has been added to the dashboard for future integration. Configure it in Settings &gt; Agents when ready.
              </p>
            </div>
          </CardContent>
        </Card>

        <div className="grid gap-6 lg:grid-cols-2">
          {/* Strengths */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Zap className="h-5 w-5 text-green-500" />
                Strengths
              </CardTitle>
              <CardDescription>
                Key advantages of OpenCode over proprietary alternatives
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {strengths.map((item) => (
                <div key={item.title} className="flex gap-3">
                  <div className="p-2 bg-muted rounded-lg h-fit">
                    <item.icon className="h-4 w-4 text-muted-foreground" />
                  </div>
                  <div>
                    <h4 className="font-medium text-sm">{item.title}</h4>
                    <p className="text-xs text-muted-foreground">{item.description}</p>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>

          {/* Limitations */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <AlertCircle className="h-5 w-5 text-amber-500" />
                Known Limitations
              </CardTitle>
              <CardDescription>
                Current weaknesses and areas for improvement
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="space-y-2">
                {limitations.map((limitation) => (
                  <li key={limitation} className="flex items-start gap-2 text-sm">
                    <X className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                    <span className="text-muted-foreground">{limitation}</span>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        </div>

        {/* Comparison Table */}
        <Card className="mt-6">
          <CardHeader>
            <CardTitle>OpenCode vs Claude Code</CardTitle>
            <CardDescription>
              Feature comparison between the two AI coding CLI tools
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="text-left py-3 px-4 font-medium">Feature</th>
                    <th className="text-left py-3 px-4 font-medium">
                      <div className="flex items-center gap-2">
                        <Terminal className="h-4 w-4 text-green-500" />
                        OpenCode
                      </div>
                    </th>
                    <th className="text-left py-3 px-4 font-medium">
                      <div className="flex items-center gap-2">
                        <Terminal className="h-4 w-4 text-purple-500" />
                        Claude Code
                      </div>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {comparisonData.map((row) => (
                    <tr key={row.feature} className="border-b last:border-0">
                      <td className="py-3 px-4 font-medium">{row.feature}</td>
                      <td className="py-3 px-4">
                        <div className="flex items-center gap-2">
                          {row.opencode === 'No' ? (
                            <X className="h-4 w-4 text-red-500" />
                          ) : row.opencode === 'Yes' ? (
                            <Check className="h-4 w-4 text-green-500" />
                          ) : null}
                          <span className="text-muted-foreground">{row.opencode}</span>
                        </div>
                      </td>
                      <td className="py-3 px-4">
                        <div className="flex items-center gap-2">
                          {row.claudecode === 'No' ? (
                            <X className="h-4 w-4 text-red-500" />
                          ) : row.claudecode === 'Yes' ? (
                            <Check className="h-4 w-4 text-green-500" />
                          ) : null}
                          <span className="text-muted-foreground">{row.claudecode}</span>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>

        {/* Quick Stats */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6">
          <Card>
            <CardContent className="pt-6">
              <div className="text-2xl font-bold text-green-600">41K+</div>
              <p className="text-xs text-muted-foreground">GitHub Stars</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="text-2xl font-bold text-blue-600">450+</div>
              <p className="text-xs text-muted-foreground">Contributors</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="text-2xl font-bold text-purple-600">75+</div>
              <p className="text-xs text-muted-foreground">AI Providers</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="text-2xl font-bold text-amber-600">MIT</div>
              <p className="text-xs text-muted-foreground">License</p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
