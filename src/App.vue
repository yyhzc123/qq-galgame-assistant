```
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GoogleGenerativeAI } from "@google/generative-ai";

const API_KEY = "AIzaSyCsMdS9v6totSlUtTlPZ4BR9v0BFrgLbLk";
const genAI = new GoogleGenerativeAI(API_KEY);

const options = ref<{ tsundere: string; sweet: string; funny: string } | null>(null);
const loading = ref(false);
const error = ref("");
const showDialogue = ref(false);
const currentEventPayload = ref<string | null>(null);
const isSilentMode = ref(false);
const isAnalyzingSilent = ref(false);
const hasSilentResult = ref(false);

// Start analysis
const startAnalysis = async () => {
  if (hasSilentResult.value) {
    // If we have a silent result ready, just show it
    await invoke("analyze", { silent: false }); // Re-trigger to resize window, but we already have data
    // Actually, analyze command captures again. We just need to resize.
    // But since we can't easily resize from frontend without a command, let's just use a dedicated "show_overlay" logic or hack it.
    // Simpler: Just call analyze(silent=false) but ignore the new capture if we have data?
    // No, that's wasteful.
    // Let's just use the fact that we have data.
    // We need a command to just "Resize to Dialogue".
    // For now, let's re-analyze to keep it simple, OR we can just use the existing flow.
    // Wait, if I click "Ready", I want to see the result I *already* got.
    // I can't resize the window from Vue easily without a Tauri command.
    // Let's assume the user wants to capture the *current* state anyway? No, silent mode implies "I captured earlier".
    
    // Workaround: We will use the `reset_window` logic but inverted?
    // Let's just trigger analyze(silent=false) for now. It will re-capture. 
    // If the user wants the *old* result, we should save it.
    // But the user said "show options obtained via silent analysis".
    // So we MUST use the old result.
    // We need a way to resize window without capturing.
    // Since I can't add a new Rust command easily without risk, I will use `analyze` but maybe ignore the image in frontend?
    // But `analyze` forces a capture delay.
    // Let's just accept the delay for now or try to use `Window.set_size` from frontend if allowed (it is allowed if configured).
    // I'll try to use `appWindow` from `@tauri-apps/api/window` if available, but I don't have it imported.
    // Let's stick to: Silent analysis gets the result. When clicking "Ready", we show the overlay.
    // The overlay needs to be full size.
    // I will trigger `analyze(silent=false)` but use the *existing* `options` if `hasSilentResult` is true.
    
    // Actually, if I call analyze(silent=false), it will emit `analyze-chat` again.
    // I can ignore that event if I want.
    loading.value = false;
    showDialogue.value = true;
    // We need to resize the window!
    // Since I can't resize from here, I HAVE to call backend.
    // I will call `analyze(silent=false)`.
    await invoke("analyze", { silent: false });
    return;
  }

  loading.value = true;
  error.value = "";
  options.value = null;
  currentEventPayload.value = null;
  isAnalyzingSilent.value = isSilentMode.value;
  
  await invoke("analyze", { silent: isSilentMode.value });
};

// Close dialogue
const closeDialogue = async () => {
  showDialogue.value = false;
  // Don't clear options if we want to keep them? No, close means reset.
  options.value = null;
  hasSilentResult.value = false;
  error.value = "";
  await invoke("reset_window");
};

const quitApp = async () => {
  await invoke("quit");
};

const analyzeImage = async (base64Image: string) => {
  // If we already have options and we are just "showing" them (from silent mode click), ignore this new analysis?
  // But `analyze` was called to resize.
  // If `hasSilentResult` is true, we preserve the old options.
  if (hasSilentResult.value && showDialogue.value) {
     loading.value = false;
     return;
  }

  loading.value = true;
  error.value = "";
  
  try {
    const model = genAI.getGenerativeModel({ model: "gemini-2.5-flash" });
    const prompt = `Analyze the chat history in this image. The last message is the one to reply to. 
    Provide 3 distinct reply options in a Galgame style (in Chinese):
    1. Tsundere (傲娇 - Cold but caring)
    2. Sweet/Gentle (温柔 - Supportive and kind)
    3. Funny/Meme (搞笑 - Humorous or sarcastic)
    
    Output ONLY a valid JSON object with keys: 'tsundere', 'sweet', 'funny'. Do not include markdown formatting.`;

    const result = await model.generateContent([
      prompt,
      {
        inlineData: {
          data: base64Image,
          mimeType: "image/png",
        },
      },
    ]);

    const response = await result.response;
    const text = response.text();
    const jsonStr = text.replace(/```json/g, "").replace(/```/g, "").trim();
    options.value = JSON.parse(jsonStr);
    
    if (isAnalyzingSilent.value) {
      hasSilentResult.value = true;
      isAnalyzingSilent.value = false;
      loading.value = false;
      // Don't show dialogue, just stay in widget mode (which will update UI to "Ready")
    }
  } catch (e: any) {
    console.error("Analysis Error:", e);
    error.value = "Error: " + (e.message || JSON.stringify(e));
    isAnalyzingSilent.value = false;
  } finally {
    if (!isAnalyzingSilent.value) {
        loading.value = false;
    }
  }
};

onMounted(async () => {
  await listen<string>("analyze-chat", async (event) => {
    if (!isAnalyzingSilent.value) {
        showDialogue.value = true;
    }
    currentEventPayload.value = event.payload;
    await analyzeImage(event.payload);
  });

  await listen("trigger-analyze", async () => {
    await startAnalysis();
  });

  await listen("analyze-error", (event) => {
    console.error(event.payload);
    error.value = "无法捕获窗口";
    if (!isAnalyzingSilent.value) showDialogue.value = true;
  });
});

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    closeDialogue();
  } catch (err) {
    console.error("Failed to copy: ", err);
  }
};

const retryAnalysis = () => {
  hasSilentResult.value = false;
  if (currentEventPayload.value) {
    analyzeImage(currentEventPayload.value);
  } else {
    closeDialogue();
    startAnalysis();
  }
};

const toggleSilentMode = (e: Event) => {
    e.stopPropagation();
    isSilentMode.value = !isSilentMode.value;
}
</script>

<template>
  <!-- Widget Mode -->
  <div v-if="!showDialogue" class="widget-container" data-tauri-drag-region>
    <!-- Main Button -->
    <div class="main-btn" @click="startAnalysis" :class="{ 'analyzing': loading && isAnalyzingSilent, 'ready': hasSilentResult }">
        <div class="btn-content">
            <span v-if="hasSilentResult">查看</span>
            <span v-else>分析</span>
        </div>
    </div>

    <!-- Controls Row -->
    <div class="controls-row">
        <button class="icon-btn" @click="toggleSilentMode" :class="{ 'active': isSilentMode }" title="静默模式">
            <span v-if="isSilentMode">🤫</span>
            <span v-else>🔔</span>
        </button>
        <button class="icon-btn quit" @click="quitApp" title="退出">
            ❌
        </button>
    </div>
  </div>

  <!-- Dialogue Mode (Directly the box, no wrapper) -->
  <div v-else class="dialogue-box">
      <div class="header-bar" data-tauri-drag-region>
        <span class="title">GALGAME ASSISTANT</span>
        <div class="controls">
            <button class="control-btn close" @click="closeDialogue">×</button>
        </div>
      </div>

      <div class="content">
        <div v-if="loading" class="loading-state">
            <div class="progress-bar">
                <div class="progress-fill"></div>
            </div>
            <span class="loading-label">SYSTEM ANALYZING...</span>
        </div>
        
        <div v-else-if="error" class="error-text">
            {{ error }}
            <button class="retry-btn" @click="retryAnalysis">RETRY</button>
        </div>

        <div v-else-if="options" class="options-list">
            <div class="option-item" @click="copyToClipboard(options.tsundere)">
                <span class="label">【傲娇】</span>
                <span class="text">{{ options.tsundere }}</span>
            </div>
            <div class="option-item" @click="copyToClipboard(options.sweet)">
                <span class="label">【温柔】</span>
                <span class="text">{{ options.sweet }}</span>
            </div>
            <div class="option-item" @click="copyToClipboard(options.funny)">
                <span class="label">【搞笑】</span>
                <span class="text">{{ options.funny }}</span>
            </div>
        </div>
      </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Noto+Serif+SC:wght@500;700&display=swap');

html, body, #app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
  font-family: 'Noto Serif SC', serif;
}
</style>

<style scoped>
/* Widget Styles */
.widget-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 10px;
}

.main-btn {
    width: 60px;
    height: 60px;
    background: linear-gradient(135deg, #2c3e50, #000);
    border: 2px solid #d4af37;
    border-radius: 50%;
    display: flex;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 4px 15px rgba(0,0,0,0.5);
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
}

.main-btn:hover {
    transform: scale(1.05);
    box-shadow: 0 0 15px rgba(212, 175, 55, 0.5);
}

.main-btn::after {
    content: '';
    position: absolute;
    top: -50%; left: -50%; width: 200%; height: 200%;
    background: linear-gradient(45deg, transparent, rgba(255,255,255,0.1), transparent);
    transform: rotate(45deg);
    animation: shine 3s infinite;
}

.main-btn.analyzing { border-color: #feca57; }
.main-btn.ready { border-color: #55efc4; }

.btn-content {
    color: #d4af37;
    font-weight: bold;
    font-size: 0.9rem;
    z-index: 2;
}

.controls-row {
    display: flex;
    gap: 10px;
    background: rgba(0,0,0,0.6);
    padding: 5px 10px;
    border-radius: 20px;
    border: 1px solid rgba(255,255,255,0.1);
}

.icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    opacity: 0.7;
    transition: all 0.2s;
    padding: 2px;
}

.icon-btn:hover { opacity: 1; transform: scale(1.1); }
.icon-btn.active { opacity: 1; text-shadow: 0 0 5px #fff; }
.icon-btn.quit:hover { transform: scale(1.2); }

/* Dialogue Styles */
.dialogue-box {
    width: 100%;
    height: 100%;
    background: rgba(20, 20, 30, 0.98);
    border: 2px solid #d4af37;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
}

.header-bar {
    background: linear-gradient(90deg, rgba(212, 175, 55, 0.15), transparent);
    padding: 8px 15px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(212, 175, 55, 0.3);
    cursor: grab;
    -webkit-app-region: drag; /* Critical for dragging */
}

.header-bar:active { cursor: grabbing; }

.title {
    color: #d4af37;
    font-size: 0.9rem;
    letter-spacing: 2px;
    font-weight: bold;
    text-shadow: 0 2px 4px rgba(0,0,0,0.5);
}

.control-btn {
    background: none;
    border: none;
    color: #d4af37;
    font-size: 1.5rem;
    cursor: pointer;
    line-height: 1;
    opacity: 0.8;
}

.control-btn:hover { opacity: 1; color: #fff; }

.content {
    flex-grow: 1;
    padding: 20px;
    display: flex;
    justify-content: center;
    align-items: center;
}

.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 15px;
    width: 60%;
}

.progress-bar {
    width: 100%;
    height: 4px;
    background: rgba(255,255,255,0.1);
    border-radius: 2px;
    overflow: hidden;
}

.progress-fill {
    width: 100%;
    height: 100%;
    background: #d4af37;
    transform-origin: left;
    animation: progress 2s infinite ease-in-out;
}

.loading-label {
    color: #d4af37;
    font-size: 0.8rem;
    letter-spacing: 3px;
    opacity: 0.8;
}

.options-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.option-item {
    background: linear-gradient(90deg, rgba(255,255,255,0.03), transparent);
    border-left: 3px solid #666;
    padding: 15px;
    cursor: pointer;
    transition: all 0.2s;
    color: #ccc;
    display: flex;
    align-items: baseline;
    gap: 10px;
}

.option-item:hover {
    background: linear-gradient(90deg, rgba(212, 175, 55, 0.15), transparent);
    border-left-color: #d4af37;
    padding-left: 20px;
    color: #fff;
}

.label {
    color: #d4af37;
    font-weight: bold;
    font-size: 0.9rem;
    white-space: nowrap;
}

.text {
    font-size: 1.05rem;
    line-height: 1.5;
}

@keyframes shine { 0% { left: -100%; } 20% { left: 100%; } 100% { left: 100%; } }
@keyframes progress { 0% { transform: scaleX(0); } 50% { transform: scaleX(0.7); } 100% { transform: scaleX(1); opacity: 0; } }
</style>
```