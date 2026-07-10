import { useState, useEffect } from 'react'
import { sessionApi } from '@/api/sessionApi'
import { SessionSummary, IdeSession } from '@/types/session'
import { useDialog } from '@/contexts/DialogContext'
import { showSuccess, showError, showWarning } from '@/utils/toast'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { decodeWorkspaceName } from '../utils/sessionFormat'

export function useIdeSessions() {
  const { showConfirm } = useDialog()
  const [workspaces, setWorkspaces] = useState<string[]>([])
  const [selectedWorkspace, setSelectedWorkspace] = useState<string | null>(null)
  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Set<string>>(new Set())
  const [workspaceSessions, setWorkspaceSessions] = useState<Map<string, SessionSummary[]>>(new Map())
  const [selectedSession, setSelectedSession] = useState<IdeSession | null>(null)
  const [loading, setLoading] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedWorkspaceHashes, setSelectedWorkspaceHashes] = useState<Set<string>>(new Set())

  // 加载 workspaces
  useEffect(() => {
    loadWorkspaces()
  }, [])

  const toggleWorkspace = async (workspaceHash: string) => {
    const newExpanded = new Set(expandedWorkspaces)

    if (newExpanded.has(workspaceHash)) {
      // 折叠
      newExpanded.delete(workspaceHash)
    } else {
      // 展开 - 加载该工作区的 sessions
      newExpanded.add(workspaceHash)
      if (!workspaceSessions.has(workspaceHash)) {
        await loadSessionsForWorkspace(workspaceHash)
      }
    }

    setExpandedWorkspaces(newExpanded)
  }

  const loadSessionsForWorkspace = async (workspaceHash: string) => {
    try {
      const data = await sessionApi.listSessions(workspaceHash)
      setWorkspaceSessions(prev => new Map(prev).set(workspaceHash, data))
    } catch (error) {
      console.error('Failed to load sessions:', error)
      showError('加载会话列表失败：' + error)
    }
  }

  const loadWorkspaces = async () => {
    try {
      setLoading(true)
      const data = await sessionApi.listWorkspaces()
      setWorkspaces(data)
    } catch (error) {
      console.error('Failed to load workspaces:', error)
      showError('加载工作区失败：' + error)
    } finally {
      setLoading(false)
    }
  }

  const handleSelectSession = async (workspaceHash: string, session: SessionSummary) => {
    // 如果点击的是当前已选中的 session，不重复加载
    if (selectedSession?.sessionId === session.sessionId) {
      return
    }

    try {
      setLoading(true)
      setSelectedSession(null) // 先清空，避免显示旧数据
      setSelectedWorkspace(workspaceHash)
      const data = await sessionApi.loadSession(workspaceHash, session.sessionId)
      setSelectedSession(data)
    } catch (error) {
      console.error('Failed to load session:', error)
      showError('加载失败：' + error)
    } finally {
      setLoading(false)
    }
  }
  const handleDeleteWorkspace = async (workspaceHash: string) => {
    const workspaceName = decodeWorkspaceName(workspaceHash)

    const confirmed = await showConfirm(
      '删除工作区',
      `确定要删除工作区 "${workspaceName}" 及其所有会话吗？\n\n此操作不可恢复！`
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // 直接删除整个工作区目录
      await sessionApi.deleteWorkspace(workspaceHash)

      // 重新加载工作区列表
      await loadWorkspaces()

      // 清空相关状态
      setExpandedWorkspaces(prev => {
        const newSet = new Set(prev)
        newSet.delete(workspaceHash)
        return newSet
      })
      setWorkspaceSessions(prev => {
        const newMap = new Map(prev)
        newMap.delete(workspaceHash)
        return newMap
      })
      if (selectedWorkspace === workspaceHash) {
        setSelectedWorkspace(null)
        setSelectedSession(null)
      }

      showSuccess(`成功删除工作区 "${workspaceName}"`)
    } catch (error) {
      console.error('Failed to delete workspace:', error)
      showError('删除工作区失败：' + error)
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteSession = async (workspaceHash: string, session: SessionSummary) => {
    const confirmed = await showConfirm(
      '删除会话',
      `确定要删除会话 "${session.title}" 吗？`
    )

    if (!confirmed) return

    try {
      await sessionApi.deleteSession(session.workspaceHash, session.sessionId)

      // 重新加载该工作区的会话列表
      await loadSessionsForWorkspace(workspaceHash)

      // 如果删除的是当前选中的 session，清空详情
      if (selectedSession?.sessionId === session.sessionId) {
        setSelectedSession(null)
      }
      showSuccess('会话已删除')
    } catch (error) {
      console.error('Failed to delete session:', error)
      showError('删除失败：' + error)
    }
  }

  const toggleWorkspaceSelection = (workspaceHash: string) => {
    const newSelected = new Set(selectedWorkspaceHashes)
    if (newSelected.has(workspaceHash)) {
      newSelected.delete(workspaceHash)
    } else {
      newSelected.add(workspaceHash)
    }
    setSelectedWorkspaceHashes(newSelected)
  }

  const toggleSelectAllWorkspaces = () => {
    if (selectedWorkspaceHashes.size === workspaces.length) {
      setSelectedWorkspaceHashes(new Set())
    } else {
      setSelectedWorkspaceHashes(new Set(workspaces))
    }
  }
  const handleBatchDeleteWorkspaces = async () => {
    if (selectedWorkspaceHashes.size === 0) {
      showWarning('请先选择要删除的工作区')
      return
    }

    const workspaceNames = Array.from(selectedWorkspaceHashes)
      .map(hash => decodeWorkspaceName(hash))
      .join('、')

    const confirmed = await showConfirm(
      '批量删除工作区',
      `确定要删除选中的 ${selectedWorkspaceHashes.size} 个工作区及其所有会话吗？\n\n工作区：${workspaceNames}\n\n此操作不可恢复！`
    )

    if (!confirmed) return

    try {
      setLoading(true)

      // 直接删除所有选中的工作区目录
      for (const workspaceHash of selectedWorkspaceHashes) {
        await sessionApi.deleteWorkspace(workspaceHash)
      }

      // 重新加载工作区列表
      await loadWorkspaces()

      // 清空相关状态
      setExpandedWorkspaces(new Set())
      setWorkspaceSessions(new Map())
      setSelectedWorkspaceHashes(new Set())
      setSelectedWorkspace(null)
      setSelectedSession(null)

      showSuccess(`成功删除 ${selectedWorkspaceHashes.size} 个工作区`)
    } catch (error) {
      console.error('Failed to batch delete workspaces:', error)
      showError('批量删除失败：' + error)
    } finally {
      setLoading(false)
    }
  }

  const handleExportSession = async (format: 'json' | 'markdown') => {
    if (!selectedSession) return

    try {
      // 从 workspaceSessions 中找到对应的 session 获取 workspaceHash
      let workspaceHash = ''
      for (const [hash, sessions] of workspaceSessions.entries()) {
        if (sessions.some(s => s.sessionId === selectedSession.sessionId)) {
          workspaceHash = hash
          break
        }
      }

      if (!workspaceHash) {
        showError('无法找到会话所属的工作区')
        return
      }

      const content = await sessionApi.exportSession(
        workspaceHash,
        selectedSession.sessionId,
        format
      )

      const ext = format === 'json' ? 'json' : 'md'
      const defaultPath = `${selectedSession.title}.${ext}`

      const filePath = await save({
        defaultPath,
        filters: [{
          name: format === 'json' ? 'JSON' : 'Markdown',
          extensions: [ext]
        }]
      })

      if (filePath) {
        await writeTextFile(filePath, content)
        showSuccess('导出成功！')
      }
    } catch (error) {
      console.error('Failed to export session:', error)
      showError('导出失败：' + error)
    }
  }

  const filteredSessions = searchQuery
    ? Array.from(workspaceSessions.values())
      .flat()
      .filter(session => session.title.toLowerCase().includes(searchQuery.toLowerCase()))
    : []

  // 获取工作区的会话列表
  const getWorkspaceSessions = (workspaceHash: string) => {
    return workspaceSessions.get(workspaceHash) || []
  }

  const loadedSessionCount = Array.from(workspaceSessions.values()).reduce(
    (total, sessions) => total + sessions.length,
    0
  )

  const selectedSessionId = selectedSession?.sessionId

  return {
    workspaces,
    selectedWorkspace,
    setSelectedWorkspace,
    expandedWorkspaces,
    workspaceSessions,
    selectedSession,
    loading,
    searchQuery,
    setSearchQuery,
    selectedWorkspaceHashes,
    filteredSessions,
    loadedSessionCount,
    selectedSessionId,
    toggleWorkspace,
    handleSelectSession,
    handleDeleteWorkspace,
    handleDeleteSession,
    toggleWorkspaceSelection,
    toggleSelectAllWorkspaces,
    handleBatchDeleteWorkspaces,
    handleExportSession,
    getWorkspaceSessions
  }
}
