const skillsGrid = document.getElementById("skillsGrid");
const statusText = document.getElementById("statusText");
const refreshButton = document.getElementById("refreshButton");
const skillDialog = document.getElementById("skillDialog");
const dialogTitle = document.getElementById("dialogTitle");
const dialogMeta = document.getElementById("dialogMeta");
const dialogPrompt = document.getElementById("dialogPrompt");
const dialogTools = document.getElementById("dialogTools");
const dialogJson = document.getElementById("dialogJson");
const copySkillButton = document.getElementById("copySkillButton");
const closeDialogButton = document.getElementById("closeDialogButton");
let selectedSkill = null;
function setStatus(text, isError = false) {
  statusText.textContent = text;
  statusText.classList.toggle("error", isError);
}
function renderEmptyState(text) {
  skillsGrid.innerHTML = "";
  const card = document.createElement("article");
  card.className = "card";
  card.innerHTML = `<h2>No Skills</h2><p>${text}</p>`;
  skillsGrid.append(card);
}
function copyText(text) {
  return navigator.clipboard.writeText(text);
}
function openSkill(skill) {
  selectedSkill = skill;
  dialogTitle.textContent = skill.name;
  dialogMeta.textContent = `v${skill.version} · ${skill.author} · ${skill.license}`;
  dialogPrompt.textContent = skill.system_prompt;
  dialogJson.textContent = JSON.stringify(skill, null, 2);
  dialogTools.innerHTML = "";
  if (skill.allowed_tools.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No tools declared.";
    dialogTools.append(item);
  } else {
    for (const tool of skill.allowed_tools) {
      const item = document.createElement("li");
      item.textContent = tool;
      dialogTools.append(item);
    }
  }
  skillDialog.showModal();
}
function renderSkillCards(skills) {
  skillsGrid.innerHTML = "";
  if (skills.length === 0) {
    renderEmptyState("Create skills with the CLI or API and refresh.");
    return;
  }
  for (const skill of skills) {
    const card = document.createElement("article");
    card.className = "card";
    const title = document.createElement("h2");
    title.textContent = skill.name;
    const description = document.createElement("p");
    description.className = "description";
    description.textContent = skill.description;
    const meta = document.createElement("p");
    meta.className = "meta";
    meta.textContent = `v${skill.version} · ${skill.author}`;
    const actions = document.createElement("div");
    actions.className = "card-actions";
    const openButton = document.createElement("button");
    openButton.className = "button";
    openButton.textContent = "Open";
    openButton.addEventListener("click", () => openSkill(skill));
    const copyButton = document.createElement("button");
    copyButton.className = "button ghost";
    copyButton.textContent = "Copy Prompt";
    copyButton.addEventListener("click", async () => {
      await copyText(skill.system_prompt);
      setStatus(`Copied prompt for ${skill.name}.`);
    });
    actions.append(openButton, copyButton);
    card.append(title, description, meta, actions);
    skillsGrid.append(card);
  }
}
async function loadSkills() {
  setStatus("Loading skills...");
  refreshButton.disabled = true;
  try {
    const response = await fetch("/skills");
    if (!response.ok) {
      throw new Error(`Unable to load skills (${response.status}).`);
    }
    const payload = await response.json();
    const skills = Array.isArray(payload.skills) ? payload.skills : [];
    renderSkillCards(skills);
    setStatus(`Loaded ${skills.length} skill(s).`);
  } catch (error) {
    renderEmptyState("The API is not reachable. Verify the server is running.");
    setStatus(error.message, true);
  } finally {
    refreshButton.disabled = false;
  }
}
refreshButton.addEventListener("click", () => {
  void loadSkills();
});
copySkillButton.addEventListener("click", async () => {
  if (!selectedSkill) {
    return;
  }
  await copyText(JSON.stringify(selectedSkill, null, 2));
  setStatus(`Copied JSON for ${selectedSkill.name}.`);
});
closeDialogButton.addEventListener("click", () => {
  skillDialog.close();
});
skillDialog.addEventListener("click", (event) => {
  const rect = skillDialog.getBoundingClientRect();
  const outside = event.clientX < rect.left ||
    event.clientX > rect.right ||
    event.clientY < rect.top ||
    event.clientY > rect.bottom;
  if (outside) {
    skillDialog.close();
  }
});
void loadSkills();
