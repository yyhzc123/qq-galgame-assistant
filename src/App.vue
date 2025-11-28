<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GoogleGenerativeAI } from "@google/generative-ai";

const apiKey = ref("");
const showSetup = ref(false);
const options = ref<Array<{ style: string; text: string }> | null>(null);
const loading = ref(false);
const error = ref("");
const showDialogue = ref(false);
const currentEventPayload = ref<string | null>(null);
const isSilentMode = ref(false);
const isAnalyzingSilent = ref(false);
const hasSilentResult = ref(false);
const promptTemplate = ref("");

const startDrag = async () => {
    await invoke("drag_window");
};

// Start analysis
const startAnalysis = async () => {
  if (hasSilentResult.value) {
    // Just expand the window to show results, don't re-analyze
    await invoke("expand_window");
    showDialogue.value = true;
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
  options.value = null;
  hasSilentResult.value = false;
  error.value = "";
  await invoke("reset_window");
};

const quitApp = async () => {
  await invoke("quit");
};

const saveApiKey = async () => {
  if (apiKey.value.trim()) {
    localStorage.setItem("gemini_api_key", apiKey.value.trim());
    showSetup.value = false;
    await invoke("reset_window");
  }
};

const analyzeImage = async (base64Image: string) => {
  if (hasSilentResult.value && showDialogue.value) {
     loading.value = false;
     return;
  }

  loading.value = true;
  error.value = "";
  
  try {
    const genAI = new GoogleGenerativeAI(apiKey.value);
    const model = genAI.getGenerativeModel({ model: "gemini-2.5-flash" });
    
    // Use loaded prompt or fallback
    const prompt = promptTemplate.value || `You are a Galgame Assistant. Analyze the chat history in the image.
    - Messages on the RIGHT side are from the USER (me).
    - Messages on the LEFT side are from OTHERS.
    
    Task: Analyze the **entire conversation context** (both your messages on the right and others' on the left) to understand the flow, relationship, and mood.
    Formulate a reply to the latest message based on this full context.
    
    Provide 3 distinct reply options in a Galgame style (in Chinese).
    Instead of fixed archetypes, dynamically choose 3 most suitable styles based on the conversation context (e.g., 傲娇, 温柔, 腹黑, 幽默, 害羞, 调皮, 高冷, etc.).
    
    Output ONLY a valid JSON array of objects, where each object has:
    - "style": The style name in Chinese (max 4 chars).
    - "text": The reply text.
    
    Example JSON format:
    [
      {"style": "傲娇", "text": "哼，谁...谁稀罕你回消息啊！"},
      {"style": "温柔", "text": "今天也很辛苦呢，要注意休息哦。"},
      {"style": "腹黑", "text": "呵呵，看来你很懂嘛~"}
    ]
    Do not include markdown formatting.`;

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
  const storedKey = localStorage.getItem("gemini_api_key");
  if (storedKey) {
    apiKey.value = storedKey;
  } else {
    showSetup.value = true;
    await invoke("setup_window");
  }

  // Load prompt template
  try {
    promptTemplate.value = await invoke("get_prompt_template");
  } catch (e) {
    console.error("Failed to load prompt:", e);
  }

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

// Helper to cycle card colors
const getCardClass = (index: number) => {
    const classes = ['pink', 'blue', 'yellow'];
    return classes[index % classes.length];
}
</script>

<template>
  <!-- Setup Screen -->
  <div v-if="showSetup" class="setup-container" data-tauri-drag-region>
    <div class="setup-card">
      <div class="setup-icon">🐾</div>
      <h3>Welcome to qqgal</h3>
      <p>Please enter your Gemini API Key to start.</p>
      
      <div class="input-group">
        <input 
            v-model="apiKey" 
            type="password" 
            placeholder="Paste API Key here" 
            class="api-input" 
            @keyup.enter="saveApiKey"
        />
      </div>

      <div class="setup-actions">
        <button @click="saveApiKey" class="save-btn" :disabled="!apiKey">Start Adventure</button>
        <a href="https://aistudio.google.com/app/apikey" target="_blank" class="link">Get API Key</a>
      </div>
    </div>
  </div>

  <!-- Widget Mode -->
  <div v-else-if="!showDialogue" class="widget-container">
    <div class="widget-glass" @mousedown="startDrag">
        <!-- Drag Handle -->
        <div class="drag-handle" @mousedown.stop="startDrag" title="Drag to move">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="#8e6e53">
                <path d="M10 9h4V6h3l-5-5-5 5h3v3zm-1 1H6V7l-5 5 5 5v-3h3v-4zm14 2l-5-5v3h-3v4h3v3l5-5zm-9 3h-4v3H7l5 5 5-5h-3v-3z"></path>
            </svg>
        </div>

        <div class="cute-badge" @click.stop="startAnalysis" @mousedown.stop :class="{ 'analyzing': loading && isAnalyzingSilent, 'ready': hasSilentResult }">
            <div class="paw-print">
                <div class="pad main"></div>
                <div class="pad toe t1"></div>
                <div class="pad toe t2"></div>
                <div class="pad toe t3"></div>
            </div>
            <div class="badge-label">
                <span v-if="hasSilentResult">查看</span>
                <span v-else>Start</span>
            </div>
        </div>

        <!-- Mini Controls (Text Based, Flat Design) -->
        <div class="mini-controls" @mousedown.stop>
            <button class="mini-btn silent" @click.stop="toggleSilentMode" :class="{ 'active': isSilentMode }" title="静默模式">
                <div class="status-dot" :class="{ 'on': isSilentMode }"></div>
                <span>Silent</span>
            </button>
            <button class="mini-btn exit" @click.stop="quitApp" title="退出">
                <span>Exit</span>
            </button>
        </div>
    </div>
  </div>

  <!-- Dialogue Mode -->
  <div v-else class="vn-container">
      <div class="vn-box">
          <div class="vn-header">
              <button class="close-btn" @click="closeDialogue">×</button>
          </div>
          <div class="vn-content">
            <!-- Loading State -->
            <div v-if="loading" class="vn-loading">
                <div class="bounce-dot"></div>
                <div class="bounce-dot"></div>
                <div class="bounce-dot"></div>
            </div>
            
            <!-- Error State -->
            <div v-else-if="error" class="vn-error">
                <p>{{ error }}</p>
                <button class="vn-retry-btn" @click="retryAnalysis">Retry</button>
            </div>

            <!-- Options List (Dynamic) -->
            <div v-else-if="options" class="vn-options">
                <div 
                    v-for="(opt, index) in options" 
                    :key="index"
                    class="vn-option-card" 
                    :class="getCardClass(index)"
                    @click="copyToClipboard(opt.text)"
                >
                    <div class="card-label">{{ opt.style }}</div>
                    <div class="card-text">{{ opt.text }}</div>
                </div>
            </div>
          </div>
      </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=M+PLUS+Rounded+1c:wght@400;700&display=swap');

html, body, #app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
  font-family: 'M PLUS Rounded 1c', sans-serif;
}
</style>

<style scoped>
/* --- Setup Styles --- */
.setup-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  background: #fffbf5; /* Light cream background for setup */
}

.setup-card {
  background: #fff;
  padding: 30px;
  border-radius: 20px;
  box-shadow: 0 10px 30px rgba(142, 110, 83, 0.1);
  text-align: center;
  width: 80%;
  max-width: 300px;
  display: flex;
  flex-direction: column;
  gap: 15px;
  border: 2px solid #fff0f5;
}

.setup-icon {
    font-size: 3rem;
    margin-bottom: -10px;
}

.setup-card h3 { 
    margin: 0; 
    color: #8e6e53; 
    font-size: 1.2rem; 
}

.setup-card p { 
    margin: 0; 
    font-size: 0.9rem; 
    color: #888; 
    line-height: 1.4;
}

.input-group {
    width: 100%;
}

.api-input {
  padding: 12px;
  border: 2px solid #eee;
  border-radius: 12px;
  font-size: 0.9rem;
  width: 100%;
  box-sizing: border-box;
  outline: none;
  transition: border-color 0.2s;
  color: #555;
  font-family: 'M PLUS Rounded 1c', sans-serif;
}

.api-input:focus {
    border-color: #ffb7b2;
}

.setup-actions {
  display: flex;
  flex-direction: column;
  gap: 15px;
  align-items: center;
  margin-top: 10px;
}

.save-btn {
  background: #ffb7b2;
  color: #fff;
  border: none;
  padding: 10px 20px;
  border-radius: 25px;
  cursor: pointer;
  font-size: 1rem;
  font-weight: bold;
  width: 100%;
  transition: all 0.2s;
  box-shadow: 0 4px 10px rgba(255, 183, 178, 0.4);
}

.save-btn:hover {
    background: #ff9e99;
    transform: translateY(-2px);
}

.save-btn:disabled { 
    background: #eee; 
    color: #aaa;
    box-shadow: none;
    cursor: not-allowed;
    transform: none;
}

.link { 
    font-size: 0.8rem; 
    color: #a2d2ff; 
    text-decoration: none; 
    border-bottom: 1px dashed #a2d2ff;
}
.link:hover { color: #74b9ff; border-bottom-style: solid; }

/* --- Widget Styles --- */
.widget-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
}

.widget-glass {
    background: rgba(255, 255, 255, 0.4);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    padding: 20px; /* Increased padding */
    border-radius: 30px;
    
    
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 15px; /* Increased gap */
    cursor: default; /* Changed from grab to default */
    min-width: 100px; /* Ensure minimum width */
    position: relative;
}

/* Drag Handle Styles */
.drag-handle {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 24px;
    height: 24px;
    display: flex;
    justify-content: center;
    align-items: center;
    cursor: move;
    opacity: 0.5;
    transition: opacity 0.2s;
    z-index: 10;
    background: rgba(255,255,255,0.5);
    border-radius: 50%;
}

.drag-handle:hover {
    opacity: 1;
    background: rgba(255,255,255,0.8);
}

.cute-badge {
    width: 65px;
    height: 65px;
    background: #fff5e6;
    border: 3px solid #ffb7b2;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 4px 10px rgba(142, 110, 83, 0.1);
    transition: all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    position: relative;
    z-index: 5;
}

.cute-badge:hover {
    transform: scale(1.1) rotate(5deg);
    border-color: #ff9e99;
}

.cute-badge.analyzing {
    border-color: #ffd93d;
    animation: wiggle 1s infinite;
}

.cute-badge.ready {
    border-color: #6ff7c1;
    background: #e0fff4;
}

/* Paw Print CSS */
.paw-print {
    position: relative;
    width: 30px;
    height: 30px;
    margin-bottom: 2px;
}

.pad {
    background: #ffb7b2;
    position: absolute;
    border-radius: 50%;
}

.cute-badge.ready .pad { background: #6ff7c1; }

.pad.main {
    width: 16px; height: 14px;
    bottom: 2px; left: 7px;
    border-radius: 40% 40% 50% 50%;
}

.pad.toe { width: 8px; height: 10px; top: 0; }
.t1 { left: 0; transform: rotate(-20deg); }
.t2 { left: 11px; top: -2px; }
.t3 { right: 0; transform: rotate(20deg); }

.badge-label {
    font-size: 0.7rem;
    color: #8e6e53;
    font-weight: bold;
}

.mini-controls {
    display: flex;
    gap: 10px;
    width: 100%;
    justify-content: center;
    /* Ensure buttons don't shrink */
    flex-shrink: 0;
    z-index: 5;
}

.mini-btn {
    background: rgba(255, 255, 255, 0.6);
    border: 1px solid #ffb7b2;
    font-size: 0.75rem; /* Slightly larger font */
    color: #8e6e53;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 6px 14px; /* Increased padding */
    border-radius: 15px;
    transition: all 0.2s;
    font-weight: bold;
    min-width: 70px; /* Ensure minimum width */
    white-space: nowrap; /* Prevent text wrapping */
}

.mini-btn:hover {
    background: #fff0f5;
    transform: translateY(-1px);
}

.mini-btn.active {
    background: #ffb7b2;
    color: #fff;
    border-color: #ffb7b2;
}

.mini-btn.exit {
    border-color: #ddd;
    color: #999;
}

.mini-btn.exit:hover {
    background: #ff6b6b;
    border-color: #ff6b6b;
    color: #fff;
}

.status-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #ddd;
}
.status-dot.on { background: #fff; }

/* --- Visual Novel Menu Styles --- */
.vn-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    background-color: transparent; 
    pointer-events: none; /* Allow clicks to pass through */
}

.vn-box {
    width: 95%;
    max-width: 400px; /* Smaller width for better UX */
    background: transparent;
    border: none;
    box-shadow: none;
    display: flex;
    flex-direction: column;
    overflow: visible; /* Allow content to flow */
    position: relative;
    pointer-events: auto; /* Re-enable clicks for the box */
}

.vn-header {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 10px;
}

.close-btn {
    background: rgba(255, 255, 255, 0.8);
    border: none;
    border-radius: 50%;
    width: 30px;
    height: 30px;
    font-size: 1.2rem;
    color: #8e6e53;
    cursor: pointer;
    box-shadow: 0 2px 5px rgba(0,0,0,0.1);
    transition: all 0.2s;
}

.close-btn:hover {
    background: #ffb7b2;
    color: #fff;
}

.vn-content {
    display: flex;
    flex-direction: column;
    gap: 15px;
}

/* Loading Animation */
.vn-loading {
    display: flex;
    gap: 8px;
    justify-content: center;
    padding: 20px;
    background: rgba(255,255,255,0.9);
    border-radius: 15px;
    width: fit-content;
    align-self: center;
    box-shadow: 0 5px 15px rgba(0,0,0,0.1);
}

.bounce-dot {
    width: 12px; height: 12px;
    background: #ffb7b2;
    border-radius: 50%;
    animation: bounce 0.6s infinite alternate;
}
.bounce-dot:nth-child(2) { animation-delay: 0.2s; background: #a2d2ff; }
.bounce-dot:nth-child(3) { animation-delay: 0.4s; background: #ffd93d; }

@keyframes bounce { to { transform: translateY(-10px); } }
@keyframes wiggle { 0%, 100% { transform: rotate(-5deg); } 50% { transform: rotate(5deg); } }

.vn-error {
    color: #ff6b6b;
    text-align: center;
    padding: 20px;
    background: rgba(255,255,255,0.9);
    border-radius: 15px;
    box-shadow: 0 5px 15px rgba(0,0,0,0.1);
}

.vn-retry-btn {
    margin-top: 10px;
    background: #ff6b6b;
    color: #fff;
    border: none;
    padding: 6px 16px;
    border-radius: 20px;
    cursor: pointer;
}

/* Options */
.vn-options {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 15px; /* More space between floating cards */
}

.vn-option-card {
    background: rgba(255, 255, 255, 0.95); /* Slightly transparent */
    backdrop-filter: blur(10px);
    border: none; /* No border initially */
    border-left: 5px solid #ddd; /* Default accent */
    border-radius: 15px;
    padding: 18px 25px;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-shadow: 0 5px 20px rgba(0,0,0,0.1); /* Softer shadow */
}

.vn-option-card:hover {
    transform: translateY(-3px) scale(1.01);
    box-shadow: 0 10px 30px rgba(0,0,0,0.15);
}

.vn-option-card.pink { border-left-color: #ffb7b2; }
.vn-option-card.pink:hover { background: #fff5f7; }
.vn-option-card.pink .card-label { color: #ff8e88; }

.vn-option-card.blue { border-left-color: #a2d2ff; }
.vn-option-card.blue:hover { background: #f0f8ff; }
.vn-option-card.blue .card-label { color: #74b9ff; }

.vn-option-card.yellow { border-left-color: #ffd93d; }
.vn-option-card.yellow:hover { background: #fffdf5; }
.vn-option-card.yellow .card-label { color: #f4c724; }

.card-label {
    font-size: 0.8rem;
    font-weight: bold;
    letter-spacing: 1px;
}

.card-text {
    color: #444;
    font-size: 1.05rem;
    line-height: 1.6;
}
</style>