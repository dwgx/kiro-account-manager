import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { FileText, RefreshCw } from 'lucide-react'
import { getThemeAccent, getThemeSurfaceStyles } from './themeAccent'
import { handleUiError } from '../../../utils/errorLogger'
import { parseFrontMatter, buildContent } from './steeringFrontMatter'
import { FileList } from './SteeringFileList'
import { Editor } from './SteeringEditor'
import { CreateModal } from './SteeringCreateModal'
import React from 'react'

function SteeringPanel({ onCountChange, projectDir }: any) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm, showSuccess } = useDialog()
  const surface = getThemeSurfaceStyles(theme)

  // 定义本地色彩系统
  const colors = {
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    btnDisabled: 'opacity-50 cursor-not-allowed grayscale',
    dialogHeader: 'border-b border-border bg-muted/30',
    info: 'bg-primary/10 ring-1 ring-primary/15'
  }

  const [files, setFiles] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedFile, setSelectedFile] = useState<any>(null)
  const [editState, setEditState] = useState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [refining, setRefining] = useState(false)
  const [creatingDefault, setCreatingDefault] = useState(false)
  const [initializingProject, setInitializingProject] = useState(false)

  const loadFiles = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<any[]>('get_steering_files', { projectDir: projectDir || null })
      setFiles(data)
      onCountChange?.(data?.length || 0)
    } catch (e) {
      handleUiError('加载 Steering 文件失败', e, { userMessage: '加载 Steering 文件失败' })
    } finally {
      setLoading(false)
    }
  }, [onCountChange, projectDir])

  useEffect(() => {
    setSelectedFile(null)
    setEditState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
    setHasChanges(false)
    loadFiles()
  }, [loadFiles])

  const handleSelect = async (file: any) => {
    if (hasChanges && !await showConfirm(t('steering.unsavedChanges'), t('steering.confirmSwitch'))) return
    setSelectedFile(file)
    const parsed = parseFrontMatter(file.content)
    setEditState({ content: parsed.body, inclusion: parsed.inclusion, filePattern: parsed.filePattern, name: parsed.name, description: parsed.description })
    setHasChanges(false)
  }

  const updateEditState = (key: string, value: any) => {
    const newState = { ...editState, [key]: value }
    setEditState(newState)
    if (selectedFile) {
      const newContent = buildContent(newState.inclusion, newState.filePattern, newState.content, newState.name, newState.description)
      setHasChanges(newContent !== selectedFile.content)
    }
  }

  const handleSave = async () => {
    if (!selectedFile) return
    setSaving(true)
    try {
      const fullContent = buildContent(editState.inclusion, editState.filePattern, editState.content, editState.name, editState.description)
      await invoke('save_steering_file', {
        fileName: selectedFile.fileName,
        content: fullContent,
        scope: selectedFile.scope,
        projectDir: projectDir || null
      })
      setFiles(files.map(f => (f.fileName === selectedFile.fileName && f.scope === selectedFile.scope) ? { ...f, content: fullContent } : f))
      setSelectedFile({ ...selectedFile, content: fullContent })
      setHasChanges(false)
    } catch (e) {
      handleUiError('保存 Steering 文件失败', e, { userMessage: t('steering.saveFailed') || '保存失败' })
    } finally {
      setSaving(false)
    }
  }
  const handleDelete = async (file: any) => {
    if (!await showConfirm(t('steering.confirmDelete'), t('steering.confirmDeleteFile', { fileName: file.fileName }))) return
    try {
      await invoke('delete_steering_file', {
        fileName: file.fileName,
        scope: file.scope,
        projectDir: projectDir || null
      })
      const newFiles = files.filter(f => !(f.fileName === file.fileName && f.scope === file.scope))
      setFiles(newFiles)
      onCountChange?.(newFiles.length)
      if (selectedFile?.fileName === file.fileName && selectedFile?.scope === file.scope) {
        setSelectedFile(null)
        setEditState({ content: '', inclusion: 'always', filePattern: '', name: '', description: '' })
        setHasChanges(false)
      }
    } catch (e) {
      handleUiError('删除 Steering 文件失败', e, { userMessage: '删除失败' })
    }
  }

  const handleCreate = async (fileName: string, inclusion: string, filePattern: string, scope: string, name: string, description: string) => {
    const fName = fileName.endsWith('.md') ? fileName : `${fileName}.md`
    const content = buildContent(inclusion, filePattern, '\n<!-- 在此添加你的 steering 规则 -->\n', name, description)
    try {
      const newFile = await invoke<any>('create_steering_file', {
        fileName: fName,
        content,
        scope,
        projectDir: projectDir || null
      })
      const newFiles = [...files, newFile]
      setFiles(newFiles)
      onCountChange?.(newFiles.length)
      setShowCreateModal(false)
      handleSelect(newFile)
    } catch (e) {
      handleUiError('创建 Steering 文件失败', e, { userMessage: t('steering.createFailed') || '创建失败' })
    }
  }

  const resolveScope = async () => {
    if (!projectDir) return 'user'
    const useProjectScope = await showConfirm(
      t('steering.scopeTitle'),
      t('steering.scopeMessage'),
      {
        confirmText: t('kiroConfig.scopeProject'),
        cancelText: t('kiroConfig.scopeUser')}
    )
    return useProjectScope ? 'project' : 'user'
  }

  const upsertFile = (nextFile: any) => {
    const nextFiles = [
      ...files.filter(file => !(file.fileName === nextFile.fileName && file.scope === nextFile.scope)),
      nextFile
    ]
    setFiles(nextFiles)
    onCountChange?.(nextFiles.length)
    handleSelect(nextFile)
  }

  const handleCreateDefault = async () => {
    setCreatingDefault(true)
    try {
      const scope = await resolveScope()
      const created = await invoke<any>('create_default_steering_file', {
        scope,
        projectDir: projectDir || null})
      upsertFile(created)
      showSuccess(t('steering.defaultCreated'), created.fileName)
    } catch (e) {
      handleUiError('创建默认 Steering 模板失败', e, { userMessage: t('steering.createDefaultFailed') || '创建失败' })
    } finally {
      setCreatingDefault(false)
    }
  }
  const handleCreateInitial = async () => {
    if (!projectDir) return
    setInitializingProject(true)
    try {
      const created = await invoke<any>('create_initial_project_steering', { projectDir })
      const createdFiles = Array.isArray(created) ? created : []
      if (createdFiles.length > 0) {
        const merged = [...files]
        for (const file of createdFiles) {
          const index = merged.findIndex(item => item.fileName === file.fileName && item.scope === file.scope)
          if (index >= 0) {
            merged[index] = file
          } else {
            merged.push(file)
          }
        }
        setFiles(merged)
        onCountChange?.(merged.length)
        handleSelect(createdFiles[0])
      }
      showSuccess(t('steering.initialCreated'), projectDir)
    } catch (e) {
      handleUiError('初始化项目 Steering 失败', e, { userMessage: t('steering.initializeFailed') || '初始化失败' })
    } finally {
      setInitializingProject(false)
    }
  }

  const handleRefine = async () => {
    if (!selectedFile) return
    setRefining(true)
    try {
      const refined = await invoke<any>('refine_steering_file', {
        fileName: selectedFile.fileName,
        scope: selectedFile.scope,
        projectDir: projectDir || null})
      upsertFile(refined)
      showSuccess(t('steering.refineSuccess'), refined.fileName)
    } catch (e) {
      handleUiError('整理 Steering 文件失败', e, { userMessage: t('steering.refineFailed') || '整理失败' })
    } finally {
      setRefining(false)
    }
  }

  const inclusionOptions = [
    { value: 'always', label: t('steering.inclusionAlways'), desc: t('steering.inclusionAlwaysDesc') },
    { value: 'auto', label: t('steering.inclusionAuto'), desc: t('steering.inclusionAutoDesc') },
    { value: 'fileMatch', label: t('steering.inclusionFileMatch'), desc: t('steering.inclusionFileMatchDesc') },
    { value: 'manual', label: t('steering.inclusionManual'), desc: t('steering.inclusionManualDesc') },
  ]

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className={`animate-spin ${accent.text}`} size={24} />
      </div>
    )
  }

  return (
    <div className="h-full flex gap-3 p-4">
      {/* 左侧列表 */}
      <FileList
        files={files}
        selectedFile={selectedFile}
        onSelect={handleSelect}
        onDelete={handleDelete}
        onRefresh={loadFiles}
        onCreate={() => setShowCreateModal(true)}
        onCreateDefault={handleCreateDefault}
        onCreateInitial={handleCreateInitial}
        creatingDefault={creatingDefault}
        initializingProject={initializingProject}
        hasProjectDir={!!projectDir}
        accent={accent}
        colors={colors}
        t={t}
      />

      {/* 右侧编辑器 */}
      <div className={`flex-1 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
        {selectedFile ? (
          <Editor
            file={selectedFile}
            editState={editState}
            hasChanges={hasChanges}
            saving={saving}
            inclusionOptions={inclusionOptions}
            onContentChange={(v: string) => updateEditState('content', v)}
            onInclusionChange={(v: string) => updateEditState('inclusion', v)}
            onFilePatternChange={(v: string) => updateEditState('filePattern', v)}
            onNameChange={(v: string) => updateEditState('name', v)}
            onDescriptionChange={(v: string) => updateEditState('description', v)}
            onSave={handleSave}
            onRefine={handleRefine}
            refining={refining}
            surface={surface}
            accent={accent}
            colors={colors}
            t={t}
          />
        ) : (
          <div className={`flex-1 flex items-center justify-center text-muted-foreground`}>
            <div className="text-center">
              <FileText size={48} className="mx-auto mb-2 opacity-30" />
              <p>{t('steering.selectToEdit')}</p>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && (
        <CreateModal
          inclusionOptions={inclusionOptions}
          onCreate={handleCreate}
          onClose={() => setShowCreateModal(false)}
          accent={accent}
          colors={colors}
          t={t}
          hasProjectDir={!!projectDir}
        />
      )}
    </div>
  )
}

export default SteeringPanel



